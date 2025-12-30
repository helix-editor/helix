# Macros

Macros are a way to record a set of actions that you want
to repeat.

`Q` starts recording the macro. Text below the statusline
will indicate the macro has begun recording. Any actions
performed after this will be recorded to the macro.
Pressing `Q` again will stop the recording, and save the
macro.

`q` will execute the macro currently stored in the default
register.

Macros can be stored and fetched from registers. By default,
if no register is given, the last recorded macro is stored
in the `@` register. `Q` and `q` will both use this register
unless otherwise specified.
