// Fail-closed verification for Microsoft's WebView2 bootstrapper.
// Keep a read-only, non-write/non-delete-shared handle until NSIS finishes execution.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wintrust.h>
#include <softpub.h>
#include <wincrypt.h>
#include <string>
#include "pluginapi.h"

namespace {
HANDLE verifiedFile = INVALID_HANDLE_VALUE;
void closeFile() {
    if (verifiedFile != INVALID_HANDLE_VALUE) CloseHandle(verifiedFile);
    verifiedFile = INVALID_HANDLE_VALUE;
}
std::wstring pop(stack_t** stack) {
    if (!*stack) return {};
    auto item = *stack;
    std::wstring value(item->text);
    *stack = item->next;
    GlobalFree(item);
    return value;
}
void push(stack_t** stack, int size, const wchar_t* text) {
    auto item = static_cast<stack_t*>(GlobalAlloc(GPTR, sizeof(stack_t) + size * sizeof(wchar_t)));
    if (!item) return;
    lstrcpynW(item->text, text, size);
    item->next = *stack;
    *stack = item;
}
bool verify(const std::wstring& path) {
    closeFile();
    const DWORD attrs = GetFileAttributesW(path.c_str());
    if (attrs == INVALID_FILE_ATTRIBUTES || (attrs & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT))) return false;
    verifiedFile = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (verifiedFile == INVALID_HANDLE_VALUE) return false;
    WINTRUST_FILE_INFO file{};
    file.cbStruct = sizeof(file);
    file.pcwszFilePath = path.c_str();
    file.hFile = verifiedFile;
    WINTRUST_DATA trust{};
    trust.cbStruct = sizeof(trust);
    trust.dwUIChoice = WTD_UI_NONE;
    trust.fdwRevocationChecks = WTD_REVOKE_WHOLECHAIN;
    trust.dwUnionChoice = WTD_CHOICE_FILE;
    trust.pFile = &file;
    trust.dwStateAction = WTD_STATEACTION_VERIFY;
    trust.dwProvFlags = WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT;
    GUID policy = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    const LONG status = WinVerifyTrust(reinterpret_cast<HWND>(INVALID_HANDLE_VALUE), &policy, &trust);
    bool microsoft = false;
    if (status == ERROR_SUCCESS) {
        auto data = WTHelperProvDataFromStateData(trust.hWVTStateData);
        auto signer = data ? WTHelperGetProvSignerFromChain(data, 0, FALSE, 0) : nullptr;
        if (signer && signer->csCertChain && signer->pasCertChain[0].pCert) {
            wchar_t organization[256]{};
            CertGetNameStringW(signer->pasCertChain[0].pCert, CERT_NAME_ATTR_TYPE, 0,
                const_cast<char*>(szOID_ORGANIZATION_NAME), organization, 256);
            microsoft = wcscmp(organization, L"Microsoft Corporation") == 0;
        }
    }
    trust.dwStateAction = WTD_STATEACTION_CLOSE;
    WinVerifyTrust(reinterpret_cast<HWND>(INVALID_HANDLE_VALUE), &policy, &trust);
    if (!microsoft) closeFile();
    return microsoft;
}
}
extern "C" __declspec(dllexport) void __cdecl VerifyMicrosoft(HWND, int size, wchar_t*, stack_t** stack, extra_parameters*) {
    const auto path = pop(stack);
    push(stack, size, verify(path) ? L"ok" : L"untrusted");
}
extern "C" __declspec(dllexport) void __cdecl CloseVerified(HWND, int, wchar_t*, stack_t**, extra_parameters*) { closeFile(); }
BOOL WINAPI DllMain(HINSTANCE, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_DETACH) closeFile();
    return TRUE;
}
