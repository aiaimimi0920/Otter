# Live resource admission

Capture, published frames, decoded frames and GPU presentation have bounded ownership and queues.
Shared pixel and byte budgets apply before allocating. Closing or pausing a source retires its
leases and in-flight work; reconnect does not revive old capture or input authority.

For investigation report actual configuration, instance count, frame size, workload duration and
process tree. Process working set alone is not GPU, memory-pressure or display-latency evidence.
Current runtime and joint checks are listed in [Live joint testing](LIVE_JOINT_TESTING.md).
