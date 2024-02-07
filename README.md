# lazy-git-checkout

Small program for quickly switching between branches, stashing and unstaging your last changes.

![demo gif](./demo.gif)

Changing branches with lgc is similar to doing:

```bash
git stash -m $CUR_BRANCH
git checkout $NEXT_BRANCH
git stash pop $LAST_NEXT_BRANCH_STASH_REF
```

but with _hopefully_ fewer keystrokes.

It also allows you to track your most used branches to avoid any `git branch | grep ...`


## Installing

**From crates.io:**

```bash
cargo install lazy-git-checkout
```

**From source:**

```bash
git clone https://github.com/manuelpepe/lazy-git-checkout.git
cd lazy-git-checkout
cargo install --path .
```

Make sure to have cargo's bin directory in your PATH.
Additionally alias for easier access:

```bash
alias lg=lazy-git-checkout
```

## Usage

```bash
$ lazy-git-commit -A .  # add project in current directory
$ lazy-git-commit       # launch ui
```

### Keybinds:

| Key           | Mode: Checkout                    | Mode: Search        | Mode: Add                        |
|---------------|-----------------------------------|---------------------|----------------------------------|
| ESC           |                                   | Set Mode: Checkout  | Set Mode: Checkout               |
| Enter         | Checkout to branch                | Checkout to branch  | Add branch to lgc known branches |
| Backspace     | Delete char                       | Delete char         | Delete char                      |
| ArrUp         | Move selection up                 | Move selection up   | Move selection up                |
| ArrDown       | Move selection down               | Move selection down | Move selection down              |
| Shift+ArrUp   | Swap selection up                 |                     |                                  |
| Shift+ArrDown | Swap selection down               |                     |                                  |
| q             | Exit app                          |                     |                                  |
| a             | Set Mode: Add                     |                     |                                  |
| ?             | Set Mode: Search                  |                     |                                  |
| r             | Remove branch from known branches |                     |                                  |
| k             | Move selection up                 |                     |                                  |
| j             | Move selection down               |                     |                                  |
| K             | Swap selection up                 |                     |                                  |
| J             | Swap selection down               |                     |                                  |

