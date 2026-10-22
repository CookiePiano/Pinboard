# Cloud folder setup

Settings > Sync offers **This PC** and **External cloud**. Pinboard synchronizes
note files through Google Drive for Desktop; it does not perform account sign-in
in this settings flow.

## This PC

Notes remain in this PC's application data directory and cloud synchronization is
off. Switching from External cloud preserves local notes, the previously selected
folder path and any existing cloud copies.

## External cloud

1. Start Google Drive for Desktop and finish signing in there.
2. In Pinboard, open Settings > Sync and select **External cloud**.
3. Choose **Connect folder** and select a folder synchronized by Google Drive for
   Desktop. If Pinboard detects a candidate, **Connect this folder** can select it.
4. Synchronization starts once connected. Use **Sync now** when needed.
5. On another PC, select the same cloud folder. Local filesystem paths may differ,
   but both must refer to the same folder in the cloud.

Canceling the picker or leaving the folder unconfigured keeps local storage active.
Use **Change folder** to choose another location. Select **This PC** to stop cloud
synchronization.

## File layout and conflict behavior

Pinboard keeps local copies and synchronizes through
`<selected-folder>/Pinboard/notes/<id>.json`. Select the parent folder on every PC,
not `Pinboard/notes` itself; selecting that nested directory duplicates the layout.

Window position, size and pin mode remain device-local. Concurrent changes to the
same note preserve both versions through a conflict copy.

## Troubleshooting

- If folder selection or access fails, verify that Google Drive for Desktop is
  running and the drive and folder are accessible.
- If notes do not appear on another PC, verify both folder selections and confirm
  that Google Drive for Desktop has completed file transfer.

Legacy direct Google Drive API settings migrate to This PC without deleting notes
or existing credentials. Connect a cloud folder to resume synchronization through
the current workflow.
