# Help a game work in tapHLE

You can use your own coding agent to improve a game. You do not need to know
how to program.

No one else is required to work on your request. You and your agent lead the
work. Agents can make mistakes. You stay in control, and you should read the
agent's summary before you publish anything.

## Before you start

You need:

1. A Windows computer.
2. A fork or clone of tapHLE.
3. A coding agent that can work in that folder.
4. Lawful access to the exact game version you want to test.

Never upload an IPA or game files to GitHub. Keep them outside the repository.
The `tapHLE_apps` folder is ignored by Git for this reason.

## If the game is on Archive.org

Give the agent the exact item link and exact IPA file name. Do not ask the
agent to search for a copy.

After the agent confirms that exact name is an original file in the live item
metadata, it may download only that file to a cache outside the tapHLE folder.
It must match the hashes before opening it. The Archive filename stays the
official name even if Windows needs a different local cache name.

Before the agent opens, inspects, or runs the IPA, it must first match:

- the Archive.org item and file name;
- the file's MD5, SHA-1, and SHA-256 hashes; and
- whether the rightsholder still sells or officially offers the game.

Only after the item, file name, and hashes match may the agent read the app ID
and version inside the IPA or run tapHLE.

If any file name, hash, app ID, or version does not match, stop. Do not use a
different local copy because it looks close. The full rules are in
`compatibility/README.md`.

## Start the work

1. Open a [Start work on a game issue](https://github.com/ephun/tapHLE/issues/new?template=game_target.yml).
   This helps people avoid doing the same work twice. It does not put the game
   in a queue for someone else.
2. Open your tapHLE folder in your coding agent.
3. Replace the bracketed parts below, then paste the prompt into the agent.

```text
I want to improve tapHLE support for [game title and exact version] on Windows.

Exact Archive.org item URL: [URL, or "none"]
Exact Archive.org IPA file name: [file name, or "none"]
Local IPA path: [path, or "download the exact named original to an external cache"]

Read AGENTS.md, HELP_A_GAME.md, compatibility/README.md,
dev-docs/agent-workflow.md, and dev-docs/app-debugging-playbook.md before
changing anything.

Create or continue the branch compat/[short-game-name]. Check the compatibility
database and dev-docs/app-notes first so you do not repeat old work.

If I gave an Archive.org item, match the exact item, original file name, MD5,
SHA-1, and SHA-256 before opening, inspecting, or running the IPA. If no local
path was supplied, download only the exact named original to an external cache
after live metadata confirms it, and treat it as opaque until its hashes match.
If Windows changes the local name, preserve the exact Archive name through
--archive-filename. Only after the hashes match may you read the app ID and
version. Stop if any check fails. If I wrote "none," do not search for an item
and do not make an Archive-linked database report. Ask me to confirm that I
authorize use of my lawful local copy for this task. Never commit the IPA,
extracted files, save data, screenshots, raw logs, or my private file paths.

Find the first thing that stops the game from working. Make the smallest useful
fix, add a focused test when possible, and run the repository checks. Keep
unfinished experiments on the compat branch. When a checkpoint is clean,
reproducible, tested, and its limitations are recorded, prepare it for merge to
trunk even if the game is not finished. Only add a compatibility database
report after the result works from a clean commit with the exact checked IPA.
If work continues into another session, update the sanitized app note so the
next agent does not repeat the same work.

Use simple status updates. Tell me when you need me to click, play, listen, or
describe what is on screen. Do not push, create a release tag, or contact
anyone unless I ask.

Follow the commit-credit rule in AGENTS.md. For OpenAI Codex, add this trailer:
Co-authored-by: OpenAI Codex <codex@openai.com>
If you are not OpenAI Codex, state the real tool identity. Do not invent a name
or email address.
```

Keep your filled-in prompt private because it contains a path on your
computer. Do not paste it into a public issue or pull request.

## Work with the agent

The agent can build code, read logs, add tests, and make commits. You may still
need to play the game, listen to sound, or describe a screen. Short, exact
answers help.

It is normal for a game to take more than one session. Useful unfinished work
belongs on `compat/<short-game-name>`. Another agent can continue from its
commits and app note later.

## Record the result in the database

The [compatibility database](https://taphle.ephun.net/compatibility) is the
public answer to "how well does this game work?" Every rating there comes from
a real tapHLE run on Windows. Recording your result is how other people find out
the game moved.

Record one when the star rating changes, in either direction — a game that got
worse is worth knowing about too. Do not record one when a rerun just repeats a
rating that is already listed for the same tapHLE commit; that only makes work
for the moderator.

**You do not need anything special to contribute.** Sign in at the database with
your GitHub account and fill in the form. That is the normal path, it works
immediately, and your agent can hand you the exact values to paste. Contributions
appear publicly once the maintainer approves them.

**If you want your agent to submit directly**, it needs an API token so it can
post without signing in as you. Ask for one in your
[Start work on a game issue](https://github.com/ephun/tapHLE/issues/new?template=game_target.yml),
and the maintainer will issue one tied to your agent. Put it in a file at
`~/.taphledb-token` (on Windows, `C:\Users\<you>\.taphledb-token`) and tell the
agent it is there. That file must stay off GitHub — never paste the token into
an issue, a pull request, a commit, or a chat log. A token only saves you a
copy-paste; it grants nothing the web form does not.

Either way, a rating of four or five stars requires a human to play the game.
An agent may claim at most three.

The database stores the rating and a one-line note of where the game stopped at
that moment, which makes each report a dated snapshot. Where a game stops *right
now*, and why, lives in `dev-docs/app-notes/<game>.md` on its compatibility
branch. Look there first when picking up work someone else started.

## Send the work back

Ask the agent to run the checks in `AGENTS.md` and explain what was really
tested. Then open a pull request from the compatibility branch.

A good pull request says:

- what changed;
- the exact game version tested;
- what now works;
- what still does not work;
- which checks passed; and
- which coding agent helped.

Partial progress is welcome when it is clearly described. Do not claim that a
game or feature works until it was tested on Windows from the commit in the
pull request.
