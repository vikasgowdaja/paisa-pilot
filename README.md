# PaisaPilot

A local-first Windows and Android expense tracker built with Rust, Tauri 2, React,
TypeScript, Tailwind CSS, shadcn-style UI components, Lucide and SQLite.

## Download and install

Visit [GitHub Releases](https://github.com/vikasgowdaja/paisa-pilot/releases).
Once the Windows release is published, download the `PaisaPilot` x64 setup
executable from its **Assets**, run it, and launch PaisaPilot from the Start menu.
End users do not need Rust, Node.js or Visual Studio.
For Android, download the signed arm64 APK from the same release's **Assets**,
install it, and use **Sign in with Google** for optional Gmail import.

**Release status:** the Windows production packaging command succeeded locally.
The installer is not yet attached or published. A draft release is not visible
to ordinary visitors. Local installer and desktop smoke tests remain required.

The local installer output folder is:

```text
src-tauri\target\release\bundle\nsis
```

The application requires Microsoft Edge WebView2 Runtime. Tauri's default
installer configuration can download the runtime if it is missing, so an
internet connection may be needed during installation. Daily use is local.
No code-signing certificate is configured; unsigned builds may trigger Windows
SmartScreen warnings. Do not disable Windows security protections.

## Features

- Monthly income, expenses, balance and recent transactions.
- Add, edit and delete income and expense transactions, with delete confirmation.
- Dates, descriptions, categories and multiple accounts.
- Search and monthly, type, account and category filters.
- CSV import/export with stable source IDs to skip duplicate imports.
- Persistent SQLite storage, initialized and migrated by Rust at startup.
- Exact integer-paise calculations and input validation in Rust.
- Forest-green desktop interface with collapsible navigation.
- Optional Gmail read-only import with local transaction extraction and a review
  queue; nothing enters the ledger until approved.

There is no cloud synchronization, automatic ledger posting, external AI
processing, advanced analytics or automated account transfers. Gmail access is
optional and requires an internet connection only to authorize and manually
sync messages.

Accounts begin at zero. Record an income transaction for an opening balance.
Account balances include all recorded dates; dashboard totals use the selected
month. No sample transactions are inserted into the real database.

## Privacy and backup

On Windows, the database is stored in the application's per-user data directory:

```text
%APPDATA%\com.paisapilot.desktop\paisapilot.sqlite3
```

The exact path is shown in **Settings**. Close every PaisaPilot window before
copying the database for backup or restoring a backup. Keep backups private.
The database is not encrypted; use Windows account security and device
encryption. On Android, the database is in the app's private application
storage; the database is not encrypted by PaisaPilot, so use Android's device
screen lock and encryption. CSV files also contain private financial
information.

Gmail OAuth access is read-only. OAuth tokens are stored in Windows Credential
Manager on Windows and are not stored in the SQLite database. Android uses
Google Play services for authorization and keeps access tokens in memory only;
tokens are not exposed to the web UI or persisted by PaisaPilot. Sync downloads
matching email content for local parsing; PaisaPilot stores message IDs,
sender/subject, connected Gmail address, extracted fields and review state
locally, not the complete message body. The Gmail API receives requests when
you sync. Transaction details are not sent to external AI services.
Disconnecting removes the saved OAuth token on Windows, revokes Android
authorization, and clears the saved account status. To revoke the Google grant
for a Windows build, remove PaisaPilot in Google Account security settings.
Disconnecting does not delete already imported transactions or Gmail review
history.

## Gmail transaction import setup

PaisaPilot uses a Google OAuth **Desktop app** client and requests only the
`https://www.googleapis.com/auth/gmail.readonly` scope. OAuth tokens are stored
in Windows Credential Manager. No Google password, OTP, client secret, or
send-mail permission is requested.

1. In [Google Cloud Console](https://console.cloud.google.com/), create/select
   a project and enable the **Gmail API**.
2. Configure the OAuth consent screen (branding and audience). For local
   testing, use **Testing** and add the Google account you will connect as a
   test user. Google may require verification before an app can be offered
   broadly because Gmail read-only is a sensitive/restricted scope.
3. Create an OAuth client ID with application type **Desktop app**. Do not
   create a Web application client or add a fixed web redirect URI. PaisaPilot
   starts a temporary loopback callback on `127.0.0.1` during Google sign-in,
   following Google's native-app OAuth flow.
4. Configure the app publisher's public OAuth client ID in a local `.env` file
   at the repository root. Create or edit `.env` in your editor and add this
   line, replacing the example value with your Desktop app Client ID:

   ```dotenv
   PAISAPILOT_GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
   ```

   Save the file as `.env` (not `.env.txt`). The Tauri/Rust build reads the
   repository-root `.env` automatically; no shell command or manual environment
   variable setup is needed. Then launch or build:

   ```powershell
   npm run desktop
   ```

   The Rust build loads this value from `.env` for development and production
   builds. `.env` is ignored by Git; do not commit it. The Client ID is public
   app configuration, not a secret. Do not add a client secret or OAuth tokens.
   For an installer, the publisher must build with its configured `.env`.
5. In **Settings → Gmail transaction import**, users select **Sign in with
   Google** and finish consent in their browser. They do not paste credentials.
   Then open
   **Mail review → Sync Gmail**. Initial sync searches the last 12 months.
   Later syncs are manual and overlap the previous two days so delayed alerts
   can be picked up safely.
6. Verify/edit each candidate's date, amount, type, category and account before
   approval. Transfers and credit-card payments are identified but kept out of
   the income/expense ledger to avoid double-counting. Refunds can be reviewed
   as income. Duplicate references and conservative exact matches are blocked.

Official references: [Gmail API](https://developers.google.com/gmail/api),
[Gmail API authorization](https://developers.google.com/gmail/api/auth), and
[OAuth 2.0 for desktop applications](https://developers.google.com/identity/protocols/oauth2/native-app).

## Android APK setup and releases

The Android app uses native Google authorization through Google Play services
and requests only Gmail's `gmail.readonly` scope. It does not use a Web OAuth
client secret, and end users do not configure OAuth credentials. Android
authorization uses the app's package name and signing-certificate fingerprint;
the public Android OAuth client is registered in Google Cloud, not bundled as a
secret in the APK. The short-lived Gmail access token is passed directly to
Rust for API requests and is not saved to disk.

### Publisher setup

1. In Google Cloud Console, enable the Gmail API and configure the OAuth consent
   screen. Add test users while the consent screen is in Testing. Gmail
   read-only access may require Google verification before broad public use.
2. Create and protect a release signing key locally. Use JDK `keytool` and keep
   the key outside the repository:

   ```powershell
   keytool -genkeypair -v -keystore "$HOME\paisapilot-release.jks" -alias paisapilot -keyalg RSA -keysize 2048 -validity 10000
   keytool -list -v -keystore "$HOME\paisapilot-release.jks" -alias paisapilot
   ```

3. Create an **Android** OAuth client in Google Cloud for package
   `com.paisapilot.desktop` and the release key's SHA-1 fingerprint. Local debug
   builds use a different signing key and need its fingerprint registered
   separately for local Gmail sign-in tests.
4. In the repository's **Settings → Secrets and variables → Actions**, add these
   repository secrets:

   - `ANDROID_KEYSTORE_BASE64` — base64 encoding of the release `.jks` file.
     To copy the value without printing it in the console, run:
     `[Convert]::ToBase64String([IO.File]::ReadAllBytes("$HOME\paisapilot-release.jks")) | Set-Clipboard`
     in PowerShell, then paste it into the secret field.
   - `ANDROID_KEYSTORE_PASSWORD` — the key and keystore password.
   - `ANDROID_KEY_ALIAS` — `paisapilot` for the example command above.

   These signing secrets are only used by GitHub Actions. Never commit or
   upload the keystore or password. Rotate/reissue the signing key only with a
   planned app migration; losing it prevents signing updates under the same app
   identity.
5. Publish a GitHub Release. The **Android APK release** workflow builds a
   signed arm64 APK and attaches it to that release. It does not run until all
   three Actions secrets and the matching Google Cloud Android OAuth client are
   configured.

### Local Android development

Install Node.js 22, Rust stable, JDK 17, Android SDK Platform 35, and Android
NDK 28.2. Configure `ANDROID_HOME` and `NDK_HOME`. From the repository:

```powershell
npm ci
npm run android:init
npm run android:prepare
npm run tauri -- android dev
```

For a signed local release APK, set `PAISAPILOT_ANDROID_KEYSTORE`,
`PAISAPILOT_ANDROID_KEYSTORE_PASSWORD`, and `PAISAPILOT_ANDROID_KEY_ALIAS` in
the build environment, then run `npm run android:build`. Never put these values
in `.env` or commit them. Android's OAuth registration must match the signing
certificate used by each debug or release build for Google sign-in.

## CSV format

Use UTF-8 CSV with these exact headers, in this order:

```csv
source_id,date,amount,kind,description,category,account
```

- Every row requires a stable, unique `source_id` of 1-200 bytes.
- Dates use `YYYY-MM-DD`; amounts are positive rupee decimals, at most two
  decimal places, without currency signs or grouping commas.
- `kind` is `income` or `expense`.
- Account and category names must already exist, and category type must match.
- Reimporting a source ID skips that row, even if its other fields changed.
- Invalid rows cancel the whole import. Maximum file size: 5 MB.
- Exports include all months and accounts, not just current filters.

Treat exports as data rather than trusted spreadsheets. Do not enable formulas
in files from untrusted sources.

## Development and packaging

Prerequisites: Node.js 22.12 or newer supported by Vite 8, Rust stable with
the MSVC target, Visual Studio C++ x64/x86 build tools, Windows SDK, and
Microsoft Edge WebView2 Runtime.

From the project directory in PowerShell:

```powershell
npm ci
npm run desktop
```

If Cargo is not on the current shell's PATH:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

Validation and production packaging:

```powershell
npm run lint
cargo test --manifest-path .\src-tauri\Cargo.toml
npm run desktop:build
```

`npm run desktop:build` runs the TypeScript/Vite production build, compiles Rust,
and creates an NSIS Windows installer. The standalone application executable
is `src-tauri\target\release\paisa-pilot.exe`. A browser-only Vite preview
cannot access the Rust database and is not the finished desktop application.

## Verification and release checklist

Verified locally on 28 October 2026:

- Frontend production build and ESLint passed.
- All three Rust tests passed: exact money/validation, CRUD/monthly totals/
  database reopen persistence, and atomic/deduplicated CSV imports.
- The production Tauri packaging command completed successfully.

Not yet verified: launching the production executable, installation on a clean
Windows 11 x64 machine, and transaction persistence across actual application
restarts. The automated session was denied access to the generated executable
and installer by organization content policy.

Before publishing the draft:

1. Run the setup executable from the local output folder.
2. Launch the installed app; add income and an expense; check monthly totals.
3. Edit a transaction, close and reopen the app, and verify the edit persisted.
4. Delete the test transaction with confirmation, restart, and verify deletion.
5. Test CSV export and reimport; duplicate source IDs must be skipped.
6. Attach the setup executable to the draft release using GitHub in your browser.
   Never attach your database, private exports, or other personal files.
7. Record a SHA-256 checksum and attach it if your organization permits.
8. Publish the release, then check its asset download link while signed out.

## Download for Windows

[**Download PaisaPilot for Windows 11 (x64)**](https://github.com/vikasgowdaja/paisa-pilot/releases/latest)

1. Open the link above.
2. Under **Assets**, download the setup `.exe`.
3. Run the installer and launch PaisaPilot from the Start menu.

No Rust, Node.js or Visual Studio installation is required.
Your transaction data stays locally on your computer.
