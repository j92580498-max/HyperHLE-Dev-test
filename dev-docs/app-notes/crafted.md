# Crafted compatibility work note

- Branch: `compat/crafted`. Reusable fixes graduated to `trunk`.
- Identity, read from `tapHLE --info` and not from the filename: display name
  `Crafted`, bundle `com.Pickl.Crafted`, version `1.0.1`, minimum OS `4.0`,
  device family iPhone, requires `accelerometer` and `opengles-1`.
- Local copy from the maintainer's collection, not Archive-backed. No public
  report has been filed.

## 2026-08-05: three stars on `10075dc6`

Reaches a gameplay loop that starts and persists. Verified with OS-level
`PrintWindow` screenshots on a clean worktree, driven by scripted input:

- Main menu → Singleplayer → Select World → Create New World → a generated
  world with the player, terrain, health bar, hotbar and touch controls.
- The world is written to disk: `Documents/saves/New World/{0,main}`.
- It responds to input in-game — pressing the movement control changes the
  scene — so the loop is live rather than a frozen frame.
- Zero panics across the run; the scene is unchanged 15 s after generation.

**It needs `--landscape-left`.** The app declares no orientation in its
`Info.plist` at all, and tapHLE only consults
`shouldAutorotateToInterfaceOrientation:` when the window is *already*
non-portrait, so nothing ever asks the app which way up it wants to be. Launched
portrait, it draws its 480-wide layout into a 320-wide screen and the
"CRAFTED PIXELS" splash is visibly cut off. This is general, not app-specific:
1167 of 1501 apps in one collection declare no `UIInterfaceOrientation`, and 366
of those ship `Default-Landscape` art. Fixing it is separate work.

## What was in the way, in the order it was hit

Each one only became visible once the previous was cleared, which is why this
took five rounds rather than one survey.

1. `alcGetIntegerv` was a `todo!()` stub. Died about five seconds in, before
   drawing anything. → `fix/openal-alc-get-integerv`.
2. `ExtAudioFileSeek` was unimplemented. → `fix/ext-audio-file-seek`.
   With these two the app reaches a stable main menu (two stars).
3. `-[NSFileManager fileExistsAtPath:isDirectory:]` computed `isDirectory` as
   `!is_file(path)`, which is **true for a path that does not exist**. The app
   checks three folders at startup, was told all three were already there,
   created none, and failed at world creation with a nonexistent parent
   directory for `Documents/saves/New World` — three screens later.
   → `fix/file-exists-is-directory`. This was the real blocker.
4. Four `NSError**` sites aborted instead of reporting: three `todo!()`s and an
   `assert!(error.is_null())` that fired even on success.
   → `fix/nserror-out-parameters`.
5. `Fs::modified` hit `unimplemented!()` for directories. Only appears on the
   *second* launch, when there is a saved world to sort by date.
   → `fix/directory-modification-time`.

## Driving it without a person at the keyboard

Worth recording because half a session went into it. Synthetic clicks via
`SetCursorPos` + `mouse_event` do **not** work: Windows refuses
`SetForegroundWindow` to a background process, so the click lands on whatever
window actually has focus and the guest sees no touch at all. Posting
`WM_LBUTTONDOWN`/`WM_LBUTTONUP` straight to the window handle does reach SDL,
but only while the window happens to hold focus, so it succeeds in some runs and
silently does nothing in others — which desynchronises a fixed-timing click
script and looks exactly like a broken app.

What works: move the real cursor, and click each target **twice**, because the
first press is consumed activating an unfocused window. Then verify each step
actually changed the screen and retry if it did not, rather than trusting a
delay. A tapHLE window is visible and interactable on the maintainer's desktop
from Claude Code — the background-desktop problem in `AGENTS.md` is AGY-specific
and does not apply here.

## Next

- Nothing has been tried beyond generating a world and moving: no mining,
  placing, saving-and-resuming, or mob interaction.
- Resuming an existing world exercises a different path — it is what turned up
  the directory-modification-time crash — and has not been re-tested since.
- Still missing statically, from `dev-scripts/demand.py app`: 29 symbols,
  mostly OpenAL capture and `UIImage{PNG,JPEG}Representation`. None blocked
  gameplay.
