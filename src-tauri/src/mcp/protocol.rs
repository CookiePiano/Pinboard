//! Newline-delimited JSON-RPC over stdio. Stdout is reserved for MCP messages.
use super::bridge::{CreateNote, ListNotes, NoteId, Operation, UpdateNote, MAX_FRAME};
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

const INSTRUCTIONS: &str = "Pinboard sticky notes on the user's own PC. You can list, search, read, create, edit, recolor, pin, show or hide, and delete notes the same way the user can in the app. Find notes with list_notes and use the returned id. Confirm with the user before deleting notes or replacing a note's whole text. Enable MCP in Pinboard Settings > Work with AI first.";
const TINT_DESCRIPTION: &str = "Color: frost (neutral), sage (green), lilac (purple), amber (yellow), rose (pink), ink (dark).";
const PIN_DESCRIPTION: &str = "Placement: normal (like other windows), top (always on top), desktop (behind other windows, on the desktop).";

const VERSIONS: [&str; 5] = ["2026-07-28", "2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

fn error(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tool_result(message: String, failed: bool) -> Value {
    json!({ "content": [{ "type": "text", "text": message }], "isError": failed })
}

fn tools() -> Value {
    let tint = json!({ "type": "string", "enum": ["frost", "sage", "lilac", "amber", "rose", "ink"], "description": TINT_DESCRIPTION });
    let pin = json!({ "type": "string", "enum": ["normal", "top", "desktop"], "description": PIN_DESCRIPTION });
    let id = json!({ "type": "string", "description": "Note id from list_notes." });
    let annotations = |read_only: bool, destructive: bool, idempotent: bool| json!({
        "readOnlyHint": read_only, "destructiveHint": destructive, "idempotentHint": idempotent, "openWorldHint": false
    });
    json!({ "tools": [
        {
            "name": "list_notes",
            "title": "List or search Pinboard notes",
            "description": "List the user's notes, most recently edited first. Each entry has id, title (first line), a short preview, color, opacity, placement, whether it is shown, and created/updated times (KST). Use query to search note text.",
            "inputSchema": { "type": "object", "additionalProperties": false, "properties": {
                "query": { "type": "string", "maxLength": 200, "description": "Optional case-insensitive text to search for." },
                "include_hidden": { "type": "boolean", "description": "Include hidden notes. Default true." },
                "limit": { "type": "integer", "minimum": 1, "maximum": 200, "description": "Maximum notes to return. Default 50." }
            } },
            "annotations": annotations(true, false, true)
        },
        {
            "name": "read_note",
            "title": "Read a Pinboard note",
            "description": "Read the full text and settings of one note.",
            "inputSchema": { "type": "object", "additionalProperties": false, "properties": { "id": id }, "required": ["id"] },
            "annotations": annotations(true, false, true)
        },
        {
            "name": "create_note",
            "title": "Create a Pinboard sticky note",
            "description": "Create a new sticky note and show it on the desktop. Supports plain text and Markdown. Size, font, color, opacity and background follow the note the user last wrote in.",
            "inputSchema": { "type": "object", "additionalProperties": false, "properties": {
                "body": { "type": "string", "minLength": 1, "maxLength": 20000, "description": "Complete note text. Use real newline characters between lines." },
                "tint": tint, "pin": pin
            }, "required": ["body"] },
            "annotations": annotations(false, false, false)
        },
        {
            "name": "update_note",
            "title": "Edit a Pinboard note",
            "description": "Change an existing note like the user would: replace its text (body) or add lines at the end (append), change color or opacity, change placement, or show/hide it. Omitted fields stay as they are. Replacing the text clears its bold/size/highlight formatting; append keeps it.",
            "inputSchema": { "type": "object", "additionalProperties": false, "properties": {
                "id": id,
                "body": { "type": "string", "maxLength": 20000, "description": "New complete text, replacing the current text." },
                "append": { "type": "string", "minLength": 1, "maxLength": 20000, "description": "Text to add as new line(s) at the end." },
                "tint": tint, "pin": pin,
                "opacity": { "type": "number", "minimum": 0, "maximum": 1, "description": "Note transparency from 0 (clear) to 1 (opaque)." },
                "visible": { "type": "boolean", "description": "true shows the note window, false hides it (the note is kept)." }
            }, "required": ["id"] },
            "annotations": annotations(false, true, false)
        },
        {
            "name": "delete_note",
            "title": "Delete a Pinboard note",
            "description": "Delete a note, exactly like the delete button in the app. It is also removed on the user's other synced devices. Confirm with the user first.",
            "inputSchema": { "type": "object", "additionalProperties": false, "properties": { "id": id }, "required": ["id"] },
            "annotations": annotations(false, true, true)
        }
    ] })
}

/// Maps a tool call to an operation. Argument shape errors are protocol errors; value
/// errors (empty text, unknown note) become tool results the model can correct.
fn parse_call(name: Option<&str>, arguments: Value) -> Result<Operation, (i32, &'static str)> {
    fn args<T: serde::de::DeserializeOwned>(value: Value, message: &'static str) -> Result<T, (i32, &'static str)> {
        serde_json::from_value(value).map_err(|_| (-32602, message))
    }
    Ok(match name {
        Some("list_notes") => Operation::ListNotes(args::<ListNotes>(arguments, "Expected optional query, include_hidden and limit")?),
        Some("read_note") => Operation::ReadNote(args::<NoteId>(arguments, "Expected id (string)")?),
        Some("create_note") => Operation::CreateNote(args::<CreateNote>(arguments, "Expected body (string), optional tint and pin, and no other arguments")?),
        Some("update_note") => Operation::UpdateNote(args::<UpdateNote>(arguments, "Expected id and optional body, append, tint, opacity, pin, visible")?),
        Some("delete_note") => Operation::DeleteNote(args::<NoteId>(arguments, "Expected id (string)")?),
        _ => return Err((-32602, "Unknown tool. Use list_notes, read_note, create_note, update_note or delete_note")),
    })
}

pub fn serve<R: BufRead, W: Write>(mut input: R, mut output: W, mut call: impl FnMut(Operation) -> Result<Value, String>) -> io::Result<()> {
    let mut initialized = false;
    let mut started = false;
    loop {
        let mut frame = Vec::new();
        let count = input.by_ref().take((MAX_FRAME + 1) as u64).read_until(b'\n', &mut frame)?;
        if count == 0 { return Ok(()) }
        let response = if frame.len() > MAX_FRAME {
            if frame.last() != Some(&b'\n') {
                loop {
                    let chunk = input.fill_buf()?;
                    if chunk.is_empty() { break }
                    let end = chunk.iter().position(|&b| b == b'\n');
                    let consumed = end.map_or(chunk.len(), |i| i + 1);
                    input.consume(consumed);
                    if end.is_some() { break }
                }
            }
            Some(error(Value::Null, -32600, "MCP message exceeds 128 KiB"))
        } else {
            match serde_json::from_slice::<Value>(&frame) {
                Err(_) => Some(error(Value::Null, -32700, "Parse error")),
                Ok(message) => {
                    let id = message.get("id").cloned();
                    let method = message.get("method").and_then(Value::as_str);
                    let valid_id = id.as_ref().is_none_or(|v| v.is_string() || v.as_i64().is_some() || v.as_u64().is_some());
                    if !message.is_object() || message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || method.is_none() || !valid_id {
                        Some(error(Value::Null, -32600, "Invalid request"))
                    } else if let Some(id) = id {
                        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
                        if !params.is_object() {
                            Some(error(id, -32602, "Params must be an object"))
                        } else {
                            let result = match method.unwrap() {
                                "initialize" if !started => {
                                    match params.get("protocolVersion").and_then(Value::as_str) {
                                        Some(version) if params.get("capabilities").is_some_and(Value::is_object)
                                            && params.get("clientInfo").is_some_and(Value::is_object) => {
                                            started = true;
                                            Ok(json!({
                                                "protocolVersion": if VERSIONS.contains(&version) { version } else { VERSIONS[0] },
                                                "capabilities": { "tools": {} },
                                                "serverInfo": { "name": "pinboard", "version": env!("CARGO_PKG_VERSION") },
                                                "instructions": INSTRUCTIONS
                                            }))
                                        }
                                        _ => Err((-32602, "Invalid initialize parameters")),
                                    }
                                }
                                "ping" => Ok(json!({})),
                                "tools/list" if initialized => Ok(tools()),
                                "tools/call" if initialized => {
                                    let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                                    parse_call(params.get("name").and_then(Value::as_str), arguments).map(|op| {
                                        let created = matches!(op, Operation::CreateNote(_));
                                        match op.validate().and_then(|()| call(op)) {
                                            Ok(value) if created => tool_result(format!("Created a new Pinboard sticky note. Note ID: {}", value["id"].as_str().unwrap_or_default()), false),
                                            Ok(value) => tool_result(serde_json::to_string_pretty(&value).unwrap_or_default(), false),
                                            Err(message) => tool_result(message, true),
                                        }
                                    })
                                }
                                "tools/list" | "tools/call" => Err((-32002, "Initialize the MCP session first")),
                                "initialize" => Err((-32600, "Session is already initialized")),
                                _ => Err((-32601, "Method not found")),
                            };
                            Some(match result {
                                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                                Err((code, message)) => error(id, code, message),
                            })
                        }
                    } else {
                        if method == Some("notifications/initialized") && started { initialized = true; }
                        None
                    }
                }
            }
        };
        if let Some(response) = response {
            serde_json::to_writer(&mut output, &response)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn replacing_or_deleting_existing_content_is_advertised_as_destructive() {
        let catalog = tools();
        for name in ["update_note", "delete_note"] {
            let tool = catalog["tools"].as_array().unwrap().iter().find(|t| t["name"] == name).unwrap();
            assert_eq!(tool["annotations"]["destructiveHint"], true);
            assert_eq!(tool["annotations"]["readOnlyHint"], false);
        }
    }

    fn session() -> String {
        concat!(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1\"}}}\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n"
        ).into()
    }

    /// Returns the responses and the bodies of created notes; other calls are recorded by tool name.
    fn run(input: String) -> (Vec<Value>, Vec<String>) {
        let mut output = Vec::new();
        let mut calls = Vec::new();
        serve(Cursor::new(input), &mut output, |op| {
            Ok(match op {
                Operation::CreateNote(note) => { calls.push(note.body); json!({ "id": uuid::Uuid::new_v4().to_string() }) }
                Operation::ListNotes(_) => { calls.push("list".into()); json!({ "notes": [] }) }
                Operation::ReadNote(_) => { calls.push("read".into()); json!({ "body": "x" }) }
                Operation::UpdateNote(u) => { calls.push(format!("update:{:?}", u.pin)); json!({ "updated": true }) }
                Operation::DeleteNote(_) => { calls.push("delete".into()); json!({ "deleted": true }) }
            })
        }).unwrap();
        (String::from_utf8(output).unwrap().lines().map(|s| serde_json::from_str(s).unwrap()).collect(), calls)
    }

    #[test]
    fn negotiate_list_and_create_note() {
        let input = session()
            + "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n"
            + "{\"jsonrpc\":\"2.0\",\"id\":\"note\",\"method\":\"tools/call\",\"params\":{\"name\":\"create_note\",\"arguments\":{\"body\":\"준비물\\n- [ ] 자료\",\"tint\":\"amber\"}}}\n";
        let (responses, created) = run(input);
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
        let names: Vec<_> = responses[1]["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["list_notes", "read_note", "create_note", "update_note", "delete_note"]);
        assert_eq!(responses[2]["id"], "note");
        assert_eq!(responses[2]["result"]["isError"], false);
        assert_eq!(created, ["준비물\n- [ ] 자료"]);
    }

    #[test]
    fn reject_uninitialized_unknown_tools_and_bad_arguments() {
        let (responses, _) = run("{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"tools/list\"}\n".into());
        assert_eq!(responses[0]["error"]["code"], -32002);
        let (responses, created) = run(session()
            + "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"delete_note\",\"arguments\":{}}}\n"
            + "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"create_note\",\"arguments\":{\"body\":\"hello\",\"id\":\"existing\"}}}\n"
            + "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"create_note\",\"arguments\":{\"body\":\" \"}}}\n");
        assert!(created.is_empty());
        assert_eq!(responses[1]["error"]["code"], -32602);
        assert_eq!(responses[2]["error"]["code"], -32602);
        assert_eq!(responses[3]["result"]["isError"], true);
    }

    #[test]
    fn recover_after_malformed_and_oversized_frames_without_stdout_noise() {
        let input = "x".repeat(MAX_FRAME + 20) + "\nnot-json\n[]\n"
            + "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}\n";
        let (responses, created) = run(input);
        assert!(created.is_empty());
        assert_eq!(responses.len(), 4);
        assert_eq!(responses[0]["error"]["code"], -32600);
        assert_eq!(responses[1]["error"]["code"], -32700);
        assert_eq!(responses[2]["error"]["code"], -32600);
        assert_eq!(responses[3]["id"], 9);
        assert_eq!(responses[3]["result"], json!({}));
    }

    #[test]
    fn full_note_control_tools_dispatch_and_validate() {
        let id = uuid::Uuid::new_v4().to_string();
        let call = |n: u32, name: &str, args: &str| format!("{{\"jsonrpc\":\"2.0\",\"id\":{n},\"method\":\"tools/call\",\"params\":{{\"name\":\"{name}\",\"arguments\":{args}}}}}\n");
        let input = session()
            + &call(2, "list_notes", "{\"query\":\"회의\"}")
            + &call(3, "read_note", &format!("{{\"id\":\"{id}\"}}"))
            + &call(4, "update_note", &format!("{{\"id\":\"{id}\",\"append\":\"- 우유\",\"pin\":\"top\"}}"))
            + &call(5, "delete_note", &format!("{{\"id\":\"{id}\"}}"))
            + &call(6, "update_note", &format!("{{\"id\":\"{id}\"}}"))
            + &call(7, "read_note", "{\"id\":\"../../settings.json\"}")
            + &call(8, "update_note", &format!("{{\"id\":\"{id}\",\"pin\":\"floating\"}}"))
            + "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/call\",\"params\":{\"name\":\"list_notes\"}}\n";
        let (responses, calls) = run(input);
        assert_eq!(calls, ["list", "read", "update:Some(Top)", "delete", "list"]);
        for i in 1..=4 { assert_eq!(responses[i]["result"]["isError"], false, "{i}"); }
        assert!(responses[1]["result"]["content"][0]["text"].as_str().unwrap().contains("notes"));
        assert_eq!(responses[5]["result"]["isError"], true);
        assert_eq!(responses[6]["result"]["isError"], true);
        assert_eq!(responses[7]["error"]["code"], -32602);
        assert_eq!(responses[8]["result"]["isError"], false);
        let tools = tools();
        let delete = tools["tools"].as_array().unwrap().iter().find(|t| t["name"] == "delete_note").unwrap();
        assert_eq!(delete["annotations"]["destructiveHint"], true);
    }
}
