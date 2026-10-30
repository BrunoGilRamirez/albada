// Reviewed default template for JOI-ProjectLife exports (MVP).
//
// Safe, minimal layout. This file:
//   - imports nothing (no arbitrary module/package resolution);
//   - performs no network access and no shell/process access;
//   - explicitly uses the bundled, OFL-licensed Libertinus Serif family.
//
// The compiler serves this file at the fixed virtual path
// `/templates/default.typ`; the generated export source imports that path. A
// template name can never influence where this source resolves, so template
// imports cannot escape the sandbox root.

#set page(
  paper: "a4",
  margin: (x: 2.2cm, y: 2.6cm),
  numbering: "1 / 1",
)

#set text(font: "Libertinus Serif", size: 11pt)
#set par(justify: false)
