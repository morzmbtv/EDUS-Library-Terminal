# Existing EDUS design

Shared tokens live in `src/shared/styles/tokens.css`; `app.css` contains existing responsive layout rules. SystemHeader and TerminalBackButton are shared across workflows. The primary device is 1280×800 touch. No page zoom or global scaling is used.

Card animation and EDUS logo are local assets. The packaged Noto Sans variable font includes Kazakh glyphs and retains its OFL license. Icons use one pinned Lucide Vue package.

Return shows active reader loans before scanning; selected state modifies the temporary basket only. Footer actions remain visible and the loan list scrolls. Card has the `или` divider and separate Face alternative. Search field opens the existing touch keyboard without a separate keyboard icon.
