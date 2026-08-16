# Warlords: Call to Arms (iPhone) compatibility work note

This is the **iPhone** build, `greyhoundgames.warlordsapp`. The iPad build,
`greyhoundgames.warlordshdapp`, is a separate app with its own record and note
(`warlords-hd.md`).

- Branch: `compat/warlords`.
- Artifact: `Warlords (v3.086) [Decrypted].ipa` from the maintainer's local
  collection. **Provenance is the local collection, not a verified Archive
  item**, and availability was not re-checked.
- Embedded identity (`tapHLE --info`): display name `Warlords`, bundle
  `greyhoundgames.warlordsapp`, version `3.086`, minimum OS `3.0`, iPhone.

## Highest milestone: 3-star (In game), tapHLE `e9237e15`

A campaign battle is fought and resolved, with no launch options. The route is
`dev-docs/clickmaps/warlords.json`: Play, Campaign, Start Campaign, Continue,
then a marked country, Attack, and dismiss the How To Play overlay. The
battlefield appears with its unit icons and dial, and when it is over the
campaign map returns **with the player's territory expanded and fresh attack
arrows** — which is what makes this a resolved battle rather than a screen that
merely opened.

Nothing app-specific was needed; it ran on `trunk` as it stood.

## Known fault: some text is still mirrored

On the race-select, army and battle screens, part of the text is drawn mirrored
top to bottom — the two bottom buttons on the race screen, the upgrade labels,
the How To Play overlay — while other text on the *same* screen reads
correctly. So this is not the flipped-context bug fixed for `UILabel` and
`NSString` drawing on 2026-08-16; it is a second path that still gets the flip
wrong, and this app is the clearest place to study it because both behaviours
appear side by side in one frame.

### What the mirrored strings actually are

Traced: this screen draws **six** strings through
`-[NSString drawInRect:withFont:lineBreakMode:alignment:]`, and those six are
exactly the mirrored ones. That is the same path the 2026-08-16 fix corrected —
but the fix only acts when the context's y axis is flipped, and here it is not,
so it correctly leaves these alone.

So the two cases differ by **destination, not by the transform**:

- A view's layer bitmap is flipped again by the compositor on its way to a
  texture, so text drawn into it has to be written the other way up. That is the
  case the fix handles.
- A bitmap context the app made itself, and uploads as a texture, is not flipped
  by anything. Text drawn into it must be written the right way up, and is not.

**One attempted fix was wrong and is recorded so it is not tried again**:
applying the same band flip inside `CGContextShowGlyphsAtPoint` broke the text
on this screen that was already correct — the app's own Core Graphics text runs
— while leaving the six mirrored strings mirrored. Reverted.

## Next discriminator

Decide the orientation from the destination rather than from the CTM. The
question to answer first is how `CGBitmapContextDrawer` can tell a
compositor-flipped layer bitmap from an app-owned one; if the layer's bitmap
carries that flag, both cases can be served without guessing, and the six
strings here are the check for one side while any `UILabel` is the check for
the other.
