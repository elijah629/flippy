# ❌ qFlipper, ✅ `flippy`

> Admit it, qFlipper sucks.

## What!

qFlipper sucks! What could you mean... It is **the one and only** flipper
control software produce by the one and only Flipper Devices Inc! How could it
be bad!!!?!?!

### Well...

- Proprietary and _barely_ open source as the codebase (pardon my language)
  FUCKING SUCKS.
- Overcomplicated codebase
- The cli is bad, barely documented, and not worth automation.
- It's not rust (ok that was a joke, but honestly who writes a NEW application
  in C++, C, and Qt nowadays).
- Slow: They made their own protobuf RPC structure and way to send it... and
  they don't implement it right... How pitiful.
- Last updated 1 year ago to fix building itself on windows...
- The last code update was over 2 years ago!

## Why this?

To fix those problems, and make the flipper more accessible to all.

## What does this do that qFlipper doesn't?

- Fixes the above problems /\
- Fully rust reimplementation of the protobuf api, built from scraps of python
  and dusty folders.
- DB management
- You can set custom firmware channels, anything that follows the directory.json
  spec goes.
-
