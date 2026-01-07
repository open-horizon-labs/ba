# ba Status

Show current ba status for this project.

## Check if initialized

First, check if ba is initialized:
```bash
if [ -d .ba ]; then
  echo "✓ ba is initialized"
else
  echo "✗ ba not initialized - run /ba init"
  exit 0
fi
```

## Show configuration

```bash
cat .ba/config.json
```

Displays:
- Version
- ID prefix (derived from project path)

## Show issue counts

```bash
echo "Issue counts:"
ba list --json | jq '. | length' | xargs -I {} echo "  Open: {}"
ba list --all --json | jq '[.[] | select(.status == "closed")] | length' | xargs -I {} echo "  Closed: {}"
ba list --all --json | jq '[.[] | select(.status == "in_progress")] | length' | xargs -I {} echo "  In Progress: {}"
```

## Show ready queue

```bash
echo ""
echo "Ready to work:"
ba ready
```

## Check your claimed issues

If `$SESSION_ID` is available:
```bash
echo ""
echo "Your claimed issues:"
ba mine --session "$SESSION_ID"
```

## Summary format

Present information concisely:

```
✓ ba initialized

Project: ab (prefix)
Version: 2

Issues:
  Open: 12
  In Progress: 3
  Closed: 45

Ready to work: 8 issues

Your claimed: 2 issues
  ab-x7k2 (P1): Fix auth bug
  ab-y8m3 (P2): Add dashboard
```

---
Be concise. Show what matters. If not initialized, suggest `/ba init`.
