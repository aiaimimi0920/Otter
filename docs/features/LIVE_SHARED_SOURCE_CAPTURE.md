# Shared-source Live capture

The source identity and capture owner determine reuse; a consumer's crop, geometry, presentation
and input permission remain distinct. Moving or resizing a consumer does not authorize a different
source. Close/cancel terminates that consumer's work; the capture owner only releases a source once
its consumers and cleanup obligations are retired.

Use the WGC cases in [Live joint testing](LIVE_JOINT_TESTING.md) to verify actual source and output
sizes. Hardware, output and ordinary desktop observations remain separate from test fixtures.
