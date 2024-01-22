* ~~Reorder branches (move up or down with K and J)~~
* ~~Reorder branches (move up or down with Shift + ArrUp and Shift+ArrDown)~~
* ~~Denote current branch with `(*)`~~
* ~~Prevent checkout to current branch~~
* Add example GIF to README.md
* Remove leftover unwraps
* Hook on post-checkout


### Async Status Progress

#### Option 1

1. Implement `on_tick` in UI and Widgets
2. Call down `on_tick` in the main loop
3. `ChangeBranchWidget.checkout` will spawn a new tokio thread that waits until `Git.checkout` is completed.
4. `ChangeBranchWidget.on_tick` will:
    1. call `Git.poll_checkout_status`
    2. update console value
5. draw should reflect changes 

#### Improvement

* Separate ratatui from tokio:  
    1. remove `#[tokio::main]`
    2. start bg thread with tokio::Builder, use `tokio::mpsc` and `std::sync::mpsc` to communicate async w/ sync worlds
    3. run git worker on bg thread
    4. on ui render (sync thread) use only non-blocking `recv()` i.e. `try_recv()`
 
https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html#communicating-between-sync-and-async-code
https://tokio.rs/tokio/topics/bridging#sending-messages (last example)


#### More improvements

On checkout, quit terminal and pipe git output to stdout. this gives the easiest access to the output without