# /review

Dispatch a code review and orchestrate the review loop. This command creates (or resumes) a Preflight review for the current repository, launches a background review agent to discuss changes with the human reviewer, and implements requested changes across revision cycles until the review is closed.

## Step 1: Find or create a review

1. Determine the review scope by running `git log --oneline main..HEAD` (or `master..HEAD`) to understand what commits are on the current branch. If the branch has diverged from the default branch, pass the default branch name as `base_ref`.
2. Run `git status` to check for untracked files. If any appear relevant to the changes being reviewed (new source files, config files — not temp/build artifacts), ask the user whether they should be staged. Stage approved files before creating the review so it includes the complete set of changes from the start.
3. Call `find_or_create_review` via the `preflight` MCP server with:
   - `repo_path`: the absolute path to the current git repository
   - `title`: a short summary derived from the recent changes (optional)
   - `base_ref`: the base branch or ref to diff against (optional — if omitted, the server auto-detects the merge-base with the default branch)
   This will return an existing open review for the repo if one exists, or create a new one.
4. Store the returned `review_id`.
5. Call `get_review` with the `review_id` to retrieve the full review metadata and list of changed files.
6. Build a `changed_files` summary string listing each file path and its change type (added, modified, deleted). This will be passed to the review agent for context.

If review creation fails, inform the user with the error details and stop.

## Step 1.5 (optional): Annotate non-trivial changes

Ask the user if they'd like you to annotate non-trivial changes with explanations before the reviewer begins.

If the user agrees, annotations will run concurrently with the review agent (see Step 2). Prepare the annotation work now:

1. Identify files with non-trivial changes from the `changed_files` list. Skip lock files, generated files (e.g., `package-lock.json`, `Cargo.lock`), and straightforward changes (simple renames, import additions, etc.).
2. If you already have sufficient context about the changes (e.g., you just implemented them in this conversation), write annotations directly from your existing knowledge — do not re-read diffs or source files. Only use `git diff main -- <file>` or `Read` when you need additional context (e.g., resuming a review you didn't implement, or changes made earlier in the conversation that have fallen out of context).
3. For each non-trivial file, call `create_thread` via the `preflight` MCP server with:
   - `review_id`: the current review ID
   - `file_path`: the file being annotated
   - `line_start` / `line_end`: the relevant line range
   - `body`: a concise explanation of what the change does and why
   - `origin`: `"AgentExplanation"`
4. Keep annotations brief and focused on intent, not line-by-line narration.

If the user declines or skips, proceed directly to Step 2.

## Step 2: Gather design context and launch the review agent

1. Check for design or plan documents that provide architectural context for the changes. Look in `.docs/plans/` and any other obvious locations. If found, read the relevant document(s) and build a `design_context` string summarizing the design intent.
2. Launch the review agent and annotations concurrently:
   - Use the **Task tool** to launch the `review-agent` as a background task. Pass it:
     - `review_id` -- the UUID of the review
     - `changed_files` -- the summary string from Step 1
     - `design_context` -- (if available) the design/plan content or a summary of it
   - If annotations were requested in Step 1.5, perform them now (in the foreground) while the review agent starts up in the background. This way the reviewer can start browsing the diff and leaving comments immediately while annotations trickle in.
3. Tell the user the review is live and provide the URL:
   ```
   Review is ready for discussion at: http://127.0.0.1:3000/reviews/{review_id}
   ```
4. Explain that the review agent is running in the background and will respond to comments. The user can open the Preflight UI, leave comments, and discuss changes. When they are satisfied, they can click "Request Revision" in the UI to signal that the agent should compile a change list.
5. Yield control back to the user. They regain their session and can continue working or wait.

## Step 3: Wait for agent results

When the background review agent task completes, inspect its output:

- **If the output contains `REVIEW_CLOSED`:** The reviewer closed the review. Inform the user that the review is complete and stop. No further action is needed.
- **If the output contains a `## Requested Changes` section:** This is a change list. Proceed to Step 4.
- **If the agent task failed or returned an error:** Inform the user of the failure and stop. Include any error details from the task output.

## Step 4: Implement changes

Parse the change list returned by the review agent. Each item follows this format:

```
1. [file_path:line(s)] Description of the change
```

For each requested change:

1. Read the relevant file and lines to understand the current code.
2. Implement the change using the appropriate tool (`Edit` for modifications, `Write` for new files, `Bash` for deletions or renames).
3. Verify the change is correct by re-reading the modified section.

After all changes are implemented:

1. Call `submit_revision` via the `preflight` MCP server with:
   - `review_id`: the current review ID
   - `message`: a brief summary of what was changed in this revision (e.g., "Renamed process_data to validate_input, extracted helper functions, removed unused imports")
2. Inform the user that the revision has been submitted.

## Step 5: Re-launch the agent

Go back to **Step 2** to start the next review-discussion-revision cycle. The review agent will pick up the new revision's diff and the reviewer can continue providing feedback.

## Exit condition

The loop ends when the review agent returns `REVIEW_CLOSED`, which means the reviewer has closed the review in the Preflight UI. At that point:

1. Inform the user that the review is complete.
2. Summarize what was accomplished across all revision cycles (number of revisions, key changes made).

## Error handling

- **Review creation fails:** Inform the user with the error message. Check that the Preflight server is running (`preflight` binary or `just run`) and that the repository has uncommitted changes to review.
- **Review agent fails:** Inform the user that the background agent encountered an error. Include the error output. Offer to re-launch the agent or abort.
- **Change implementation fails:** If a specific change cannot be applied (e.g., file not found, conflicting edits), skip it, note the failure, and continue with remaining changes. Include skipped changes in the revision message so the reviewer is aware.
- **MCP server unreachable:** Inform the user that the Preflight server appears to be down and suggest running `just run` or `preflight serve` to start it.
