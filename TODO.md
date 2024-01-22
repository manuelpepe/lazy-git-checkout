* ~~Reorder branches (move up or down with K and J)~~
* ~~Reorder branches (move up or down with Shift + ArrUp and Shift+ArrDown)~~
* ~~Denote current branch with `(*)`~~
* ~~Prevent checkout to current branch~~
* Add example GIF to README.md
* Remove leftover unwraps
* Hook on post-checkout


### Async Status Progress

1. Implement `on_tick` in UI and Widgets
2. Call down `on_tick` in the main loop
3. `ChangeBranchWidget.checkout` will spawn a new tokio thread that waits until `Git.checkout` is completed.
4. `ChangeBranchWidget.on_tick` will:
    1. call `Git.poll_checkout_status`
    2. update console value
5. draw should reflect changes 