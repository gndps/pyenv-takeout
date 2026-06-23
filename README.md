# pyenv-takeout

Manage Python virtual environments in `~/pyenvs/` with deterministic path-based names. Venvs are named `<basename>-<sha1>` so the same project always maps to the same venv regardless of where you activate from.

## Install

```sh
brew install gndps/tap/pyenv-takeout
```

## Shell integration

Add to `~/.bash_profile` or `~/.zshrc`:

```sh
eval "$(pyenv-takeout init-shell)"
```

Then use:

```sh
es          # activate venv for current directory
es myenv    # activate named venv
ed          # deactivate and delete venv for current directory
els         # list all venvs
```

## Commands

| Command | Description |
|---------|-------------|
| `create [name]` | Create a venv (defaults to cwd-derived name) |
| `delete [name]` | Delete a venv |
| `list` | List all venvs in `~/pyenvs/` |
| `activate [name]` | Print activation command for eval |
| `path [dir]` | Print the venv path for a directory |
| `install [name]` | Create venv if needed, then `pip install -r requirements.txt` |
| `init-shell` | Print shell functions (`es`, `ed`, `els`) |

## Venv naming

```
myproject-a1b2c3d4
└─ basename + first 8 chars of sha1 of absolute path
```
