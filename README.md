# Motia CLI (Rust)

CLI for scaffolding Motia projects with iii integration.

## Development

### Setup Git Hooks

To automatically rebuild template zips when committing template changes:

```bash
# Install the pre-commit hook
cp scripts/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### Manually Build Template Zips

```bash
./scripts/build-template-zips.sh
```

### Run Locally

```bash
# With local templates (development)
cargo run -- --template-dir=./templates

# With a specific template
cargo run -- --template-dir=./templates -t quickstart
```

## Template Structure

Templates are organized as:

```
templates/
├── template.yaml          # Root manifest (lists templates + language_files)
├── quickstart/
│   ├── template.yaml      # Template manifest (name, files, requires, optional)
│   └── src/...
├── quickstart.zip         # Auto-generated zip for remote fetching
└── blank/
    ├── template.yaml
    └── ...
```

### Root `template.yaml`

```yaml
templates:
  - quickstart
  - blank

language_files:
  common:
    - ".env"
    - ".gitignore"
  python:
    - "*_step.py"
    - "requirements.txt"
  typescript:
    - "*.step.ts"
    - "tsconfig.json"
  javascript:
    - "*.step.js"
  node: # JS or TS
    - "package.json"
```

### Template-specific `template.yaml`

```yaml
name: Quickstart
description: A starter template
version: 0.1.0
requires:
  - typescript
optional:
  - javascript
  - python
files:
  - package.json
  - src/start.step.ts
  - src/python_step.py
```

## Remote Templates

For remote fetching (production), the CLI:

1. Fetches `template.yaml` to get the list of templates
2. Fetches `{template_name}.zip` for the selected template
3. Extracts and filters files based on selected languages

The GitHub Action automatically rebuilds zips when template files change.
