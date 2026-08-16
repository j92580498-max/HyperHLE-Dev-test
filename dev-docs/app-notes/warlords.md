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

## Next discriminator

Find which drawing call produces the mirrored strings here. Trace the
`NSString` drawing family and `CGContextShowText`-style calls during the race
screen, and compare a mirrored label with a correct one on the same screen: the
difference will be the path that needs the same band flip that
`draw_at_point`/`draw_in_rect` now apply.
