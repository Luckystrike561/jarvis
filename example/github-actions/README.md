# GitHub Actions Example

This example demonstrates Jarvis's GitHub Actions discovery and local execution
features. Run it with:

```bash
jarvis -p example/github-actions
```

Jarvis will discover all `.yml` files under `.github/workflows/` and display them
in the **GitHub Actions** category, showing each workflow's name, trigger events,
and jobs.

---

## Workflows

| File | Triggers | Jobs | `act` event picked |
|------|----------|------|--------------------|
| `hello.yml` | `workflow_dispatch`, `push` | `greet` | `workflow_dispatch` |
| `deploy.yml` | `workflow_dispatch` | `deploy` | `workflow_dispatch` |
| `pr-check.yml` | `pull_request` | `validate` | `pull_request` |
| `build.yml` | `push`, `pull_request` | `build` | `push` |
| `lint.yml` | `push`, `pull_request` | `clippy` | `push` |

The **act event picked** column shows what Jarvis passes to `act` when you press
Enter on a workflow. The priority is:

```
workflow_dispatch → push → pull_request → schedule → (first trigger)
```

This ensures `workflow_dispatch` workflows run as intended locally, and push-based
workflows don't accidentally fire as a PR check.

---

## Execution Modes

When you select a workflow in the TUI, Jarvis tries each in order:

1. **`act` (local, recommended)** — Runs the workflow in Docker. Jarvis automatically
   picks the correct trigger event via `select_act_event()`.

   ```
   act workflow_dispatch -W .github/workflows/hello.yml
   act push             -W .github/workflows/build.yml
   act pull_request     -W .github/workflows/pr-check.yml
   ```

2. **`gh workflow run` (remote)** — Falls back to GitHub if `act` is not installed.
   Requires the workflow to have a `workflow_dispatch` trigger and a valid GitHub
   remote.

3. **Informational echo** — If neither tool is available, prints a message explaining
   what to install.

---

## Prerequisites

| Tool | Purpose | Install |
|------|---------|---------|
| `act` | Run workflows locally | [github.com/nektos/act](https://github.com/nektos/act) |
| Docker | Required by `act` | [docs.docker.com](https://docs.docker.com/get-docker/) |
| `gh` | Remote trigger fallback | [cli.github.com](https://cli.github.com) |

`act` and `gh` are both available via `devbox shell` in the Jarvis repo.
