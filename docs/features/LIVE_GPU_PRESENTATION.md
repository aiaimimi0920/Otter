# Live presentation

Live uses WGC capture and a native presentation surface. New frames can replace pending work,
but loss of source identity, input release and teardown cannot be coalesced away. The graphics path
can recover or use a compatible presentation backend independent of peer versions.

When tuning, separate capture/delivery/decode/upload/present timing and account for the EXE, hook,
Loom and helper processes. Isolated GPU pixel readback is not monitor FPS or physical input proof.
See [joint testing](LIVE_JOINT_TESTING.md) and [resource admission](LIVE_RESOURCE_ADMISSION.md).
