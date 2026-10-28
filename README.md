# PaisaPilot

A local-first Windows expense tracker built with Rust, Tauri 2, React,
TypeScript, Tailwind CSS, shadcn-style UI components, Lucide and SQLite.

## Download and install

Visit [GitHub Releases](https://github.com/vikasgowdaja/paisa-pilot/releases).
Once the Windows release is published, download the `PaisaPilot` x64 setup
executable from its **Assets**, run it, and launch PaisaPilot from the Start menu.
End users do not need Rust, Node.js or Visual Studio.

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

There is no cloud synchronization, Gmail integration, automatic UPI ingestion,
AI processing, advanced analytics or automated account transfers.

Accounts begin at zero. Record an income transaction for an opening balance.
Account balances include all recorded dates; dashboard totals use the selected
month. No sample transactions are inserted into the real database.

## Privacy and backup

The database is stored in the application's per-user Windows data directory:

```text
%APPDATA%\com.paisapilot.desktop\paisapilot.sqlite3
```

The exact path is shown in **Settings**. Close every PaisaPilot window before
copying the database for backup or restoring a backup. Keep backups private.
The database is not encrypted; use Windows account security and device
encryption. CSV files also contain private financial information.

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
