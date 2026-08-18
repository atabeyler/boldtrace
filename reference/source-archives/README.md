# BOLDTRACE Source Archives

This directory is reserved for source archives owned by the BOLD project and used only as product-development references.

## Expected archives

Place the two reference ZIP files here without extracting them into the production application tree:

- `anatolia-bold-sim.zip` — reference for authentication, registration, localization, visual language, footer and product interaction patterns.
- `anatolia-bold-q.zip` — reference for dashboard shell, navigation, settings, notifications, profile/session controls, sidebar behavior and operational UI patterns.

## Usage rules

These archives are reference material only. They must not be included in the web build, Rust workspace, Docker image or production runtime. BOLDTRACE implementations should reuse appropriate interaction patterns while keeping market-intelligence terminology, data models and product identity native to BOLDTRACE.

Do not copy credentials, signing keys, keystores, environment secrets, tokens or private deployment configuration from any reference project.

## Current BOLDTRACE product direction

BOLDTRACE V1 combines the existing Rust market-intelligence backend with a six-language web application. Product areas include authentication, Command Center, Intelligence Terminal, Engine Matrix, Performance Center, Learning Center, Market Scanner, Alerts, History, System Health and Settings. Shared branding, footer, language selection, profile, notification and security/session controls are part of the application shell.
