# Privacy

Pinboard stores notes, settings, images and exports as unencrypted local files.
It has no application analytics or crash-upload service. It is not a password vault.
Your OS, disk encryption and account permissions protect these files.

External cloud synchronization is disabled until you choose a folder. The cloud
client managing that folder controls transmission, authentication and its own
privacy policy. Anyone permitted to modify that shared folder can modify its notes.
Legacy direct Google Drive credentials use the operating system's credential store.

AI/MCP integration is disabled until explicitly configured and enabled on this PC.
Connected clients may read, create, edit and delete notes. Their providers may
receive note content under their own privacy terms. Disabling connections stops
new authorized MCP operations; it cannot recall information already shared.
Client configuration is changed only by the explicit connection action.

Clicking a web link opens your browser, which may transmit information to the
destination. The Windows installer may obtain WebView2 from Microsoft when it is
missing. Windows certificate verification can contact certificate/revocation
services. WebView2 and your browser follow Microsoft's privacy settings and policy:
https://privacy.microsoft.com/privacystatement

Login startup is opt-in. Removing Pinboard preserves notes by default; the installed
uninstaller has a separate, unchecked option to remove local data. Exported ZIPs,
cloud copies and data held by other services are not deleted by uninstallation.
