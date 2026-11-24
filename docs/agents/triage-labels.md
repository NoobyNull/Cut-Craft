# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

## Creating the labels

Only `wontfix` exists in the repo so far. Applying a label that doesn't exist fails, and the skills don't create labels, so create the other four once before the first triage:

```sh
gh label create needs-triage    --repo NoobyNull/Cut-Craft --description "Maintainer needs to evaluate this issue"
gh label create needs-info      --repo NoobyNull/Cut-Craft --description "Waiting on reporter for more information"
gh label create ready-for-agent --repo NoobyNull/Cut-Craft --description "Fully specified, ready for an AFK agent"
gh label create ready-for-human --repo NoobyNull/Cut-Craft --description "Requires human implementation"
```

The GitHub connector tools can't create labels, so a project thread asks Matthew to run these (or add the labels in the repo's Labels page) if they're missing.

Edit the right-hand column to match whatever vocabulary you actually use.
