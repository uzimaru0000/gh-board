---
name: gh-board
description: Manage GitHub Projects V2 kanban boards from the CLI. List projects, view boards, create/move/archive cards, manage comments and fields. All commands output JSON.
---

# gh-board CLI Skills

gh-board provides CLI subcommands for programmatic access to GitHub Projects V2. All commands output JSON to stdout.

## Authentication

Uses `gh auth token` for authentication. Ensure you're logged in:
```
gh auth login
gh auth refresh -s project
```

## Commands

### Project

```bash
# List projects for current user
gh board project list

# List projects for an org or user
gh board project list --owner <LOGIN>

# View project details
gh board project view <NUMBER> [--owner <LOGIN>]
```

### Board

```bash
# Get full board as JSON (columns, cards, fields)
gh board board view <NUMBER> [--owner <LOGIN>] [--group-by <FIELD_NAME>]
```

### Item

```bash
# List all items in a project (flat list with custom field values)
gh board item list <NUMBER> [--owner <LOGIN>]
```

Each item includes `custom_fields` containing all project field values (Priority, Sprint, etc.).

### Card

```bash
# Create a draft issue on the project
gh board card create <NUMBER> --title <TITLE> [--body <BODY>] [--owner <LOGIN>] [--status <STATUS_NAME>]

# Create a real issue and add it to the project
gh board card create-issue <NUMBER> --repo <OWNER/REPO> --title <TITLE> [--body <BODY>] [--owner <LOGIN>] [--status <STATUS_NAME>]

# Get card details (Issue only, by content node ID)
gh board card get <CONTENT_ID>

# Edit a card (update title/body)
gh board card edit <CONTENT_ID> --type <draft|issue|pr> --title <TITLE> [--body <BODY>]

# Move a card (update a field value)
gh board card move <PROJECT_ID> <ITEM_ID> --field-id <FIELD_ID> --value <VALUE> [--value-type <single_select|iteration|text|number|date>]

# Archive a card
gh board card archive <PROJECT_ID> <ITEM_ID>
```

### Comment

```bash
# List comments on an issue/PR
gh board comment list <CONTENT_ID>

# Add a comment
gh board comment add <CONTENT_ID> --body <BODY>

# Update a comment
gh board comment update <COMMENT_ID> --body <BODY>
```

### Field

```bash
# List field definitions (SingleSelect options, Iteration, Text, Number, Date)
gh board field list <NUMBER> [--owner <LOGIN>]
```

### Label

```bash
# List labels for a repository
gh board label list --repo <OWNER/REPO>
```

### Assignee

```bash
# List assignable users for a repository
gh board assignee list --repo <OWNER/REPO>
```

### Skill

```bash
# Output this document
gh board skill
```

## Common Workflows

### 1. View a project board

```bash
# Find the project number
gh board project list --owner myorg

# Get the board
gh board board view 5 --owner myorg
```

### 2. Create an issue and set status

```bash
# Create a draft issue with initial status
gh board card create 5 --owner myorg --title "Fix login bug" --body "Details..." --status "In Progress"

# Or create a real issue in a repo
gh board card create-issue 5 --owner myorg --repo myorg/myrepo --title "Fix login bug" --status "Todo"
```

### 3. Move a card to a different status

```bash
# First, get field definitions to find field_id and option_id
gh board field list 5 --owner myorg

# Then move the card (use the option_id from field list)
gh board card move <PROJECT_ID> <ITEM_ID> --field-id <FIELD_ID> --value <OPTION_ID>
```

### 4. Add a comment to an issue

```bash
# Get the board to find content_id
gh board board view 5 --owner myorg

# Add a comment using the content_id from a card
gh board comment add <CONTENT_ID> --body "This is my comment"
```

## Node IDs

Many commands use GitHub GraphQL node IDs. These are opaque strings like `PVT_kwHOAB...`. You can obtain them from the JSON output of other commands:

- `project_id`: from `gh board project view` → `.id`
- `item_id`: from `gh board board view` → `.columns[].cards[].item_id` or `gh board item list` → `[].item_id`
- `content_id`: from `gh board board view` → `.columns[].cards[].content_id`
- `field_id`: from `gh board field list` → `[].id` (for SingleSelect) or nested `.id`
- `option_id`: from `gh board field list` → SingleSelect's `.options[].id`
