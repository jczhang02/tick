# Project icons

Projects can have a custom icon, set with the `icon` property. It holds one of
the icon names below, or `null` for the default icon (`#`).

On update, pass `null` to reset to the default icon, or omit `icon` to keep the
current one.

Setting an icon depends on the user's plan, shown by the `project_icons` plan
limit. On workspace projects the workspace's plan applies. Without it, setting
an icon fails with `PREMIUM_ONLY`. Resetting to the default icon always works.

We'll add more icons over time. If a client gets an icon name it doesn't know,
it should show the default icon. It must not send `null` back unless the user
reset the icon.

- `briefcase`
- `folder`
- `diagram`
- `headset`
- `microphone`
- `flag`
- `bullseye`
- `house`
- `heart`
- `airplane`
- `gift`
- `pin`
- `school_cap`
- `lightbulb`
- `star`
- `command_prompt`
- `laptop`
- `envelope`
- `calendar`
- `person`
- `sofa`
- `cup_and_heat_waves`
- `paintpalette`
- `broom`
- `sun_max`
- `moon`
- `beach_umbrella`
- `pizza`
- `checkmark_line_3`
- `basketball`
- `face_smiling`
- `dog`
- `dollarsign`
- `smartphone`
- `person_2`
- `chart_line_uptrend_xyaxis_in_square`
- `laboratory_flask`
- `telescope`
- `book`
- `document`
- `sparkles`
- `video`
- `paintbrush`
- `puzzlepiece_extension`
- `clock`
- `pills`
- `birthday_cake`
- `stethoscope`
- `apple`
