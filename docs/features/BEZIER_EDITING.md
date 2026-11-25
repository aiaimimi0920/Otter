# Freehand strokes, lines and Bezier curves

## Pencil and brush

Press Ctrl+E to open the main edit strip, then choose Pencil or Brush from the pencil button's dropdown. The main button remembers your selection for subsequent use.

- Pencil strokes have a constant width. Brush strokes have a slightly fuller rounded start, speed-dependent width and a finer finish. Slow mouse movement produces fuller strokes; fast movement produces lighter, thinner strokes. Width transitions are eased and edges are only lightly softened. Native pen-pressure support is not claimed.
- Both tools share a color picker but remember their own width, opacity and smoothing. Width ranges from 1–999; opacity and smoothing range from 0–100. Numeric inputs support increment/decrement steps.
- Pencil smoothing defaults to 0 to preserve the original freehand path. Brush defaults to width 16 and smoothing 50. Smoothing affects only new strokes, not existing annotations.
- Pencil retains its highlighter toggle. Other parameters remain available: opacity adjusts the base highlight intensity, smoothing still processes the path, and enabling highlighting hides no properties.
- Click to draw a dot, drag to draw a stroke, or hold Shift to constrain a straight line. Releasing commits one history entry; Escape or pointer cancellation does not commit an unfinished stroke.
- Existing annotations support selection, movement, rotation, scaling, undo, redo and rasterization.

The native brush stores its centerline and width weights, so reloading does not recompute the stroke from speed. Pencil remains constant-width.
See [`annotation_paintbrush.rs`](../../native/src/desktop/annotation_paintbrush.rs).

## Lines and Bezier curves

- Press Ctrl+E to open the main edit strip, then choose Bezier Curve from the line button's dropdown.
- Press and drag to set the start and end points. Releasing creates an initially curved cubic Bezier and automatically enters selection mode.
- White dots are endpoints or intermediate nodes; each segment has two yellow control points. Drag control points to adjust curvature independently. Moving a node translates its adjacent control points with it.
- Double-click any non-endpoint position on the curve to add a node. Exact subdivision preserves the original shape. You can move the new white node and adjust its two yellow control handles independently.
- Drag outward from the `+` beside either endpoint to extend the curve repeatedly. The entire path remains one object. Clicking `+` also adds a short segment.
- Right-click a white node to choose Delete Node. Deleting an intermediate node joins its adjacent segments and retains their outer control handles; the nearby shape may change. Deleting an endpoint removes the first or last segment. Yellow handles cannot be deleted individually, and at least one segment must remain. Deletion supports undo and redo.
- Click empty space to deselect, then click the curve to resume editing. In selection mode, dragging the curve itself moves the whole object.
- The properties strip offers stroke color, width, style and arrows. These properties can also be changed for a single selected existing line, arrow or curve.
- New straight lines additionally offer angle snapping; curves do not show this toggle. Lines and curves store separate defaults for width, style and arrows.
- Each completed drag commits one history entry, with undo and redo support. Canceling drawing or a control-point drag does not commit an unfinished shape.

The native model stores `3n+1` points: the initial point, then two control points and an endpoint per segment, up to 64 segments. It does not store a sampled polyline.
Rendering, hit testing and export use native geometry; control handles belong only to the editing layer.
See [`editor_bezier.rs`](../../native/otter-runtime/src/editor_bezier.rs)
and [`annotation_bezier.rs`](../../native/src/desktop/annotation_bezier.rs).

## Verification

The model, history and stroke timing live in `otter-runtime`; native hit testing, rendering and export live in the host. Test them separately:

```powershell
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-runtime --lib -E 'test(bezier) | test(editor::brush_motion::)'
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-native --lib --features test-canvas -E 'test(bezier) | test(paintbrush)'
```

`editor::brush_motion::` covers weight, timing and cancellation regressions in
[`editor_brush_motion.rs`](../../native/otter-runtime/src/editor_brush_motion.rs).
Filtering only for `paintbrush` in the host does not run these model tests.
Unit tests do not replace drag, cancellation, undo and export acceptance checks on the actual EXE.
Ordinary PNG projection delivers only a composited image; it does not imply synchronized editing
of each underlying annotation object on the receiver.
