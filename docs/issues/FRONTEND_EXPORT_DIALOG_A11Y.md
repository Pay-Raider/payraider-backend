# Issue: Add Accessible Loading State Text to ExportDialog

**Type:** Accessibility Enhancement  
**Severity:** Minor  
**Component:** `frontend/src/components/ExportDialog.tsx`  

## Problem

When the ExportDialog is in the exporting state (`isExporting: true`), screen reader users receive no auditory feedback about the ongoing operation. The component displays a `Loader2` icon and a progress bar, but these are purely visual elements that aren't accessible to users relying on assistive technologies.

## Current Behavior

- A Loader2 icon animates during export
- Progress bar shows visual progress (0-100%)
- No text content or ARIA live region announces export status
- Screen reader users are unaware that an operation is in progress

## Expected Behavior

- Add visible or aria-live text that announces: "Exporting data... {progress}%"
- Provide clear status updates as export progresses
- Announce completion status when export finishes

## Suggested Solution

1. Add an `aria-live="polite"` region with loading state text
2. Include progress percentage in the announcement
3. Consider adding visible status text (e.g., "Exporting... 45%") for all users

## Files to Modify

- [frontend/src/components/ExportDialog.tsx](frontend/src/components/ExportDialog.tsx#L24)

## Acceptance Criteria

- [ ] Screen reader users hear "Exporting..." when export begins
- [ ] Progress updates are announced periodically
- [ ] Completion message is announced when export finishes
- [ ] All changes pass accessibility audit (a11y tests)
- [ ] Keyboard navigation remains unaffected

## Labels

- `accessibility`
- `frontend`
- `enhancement`
- `minor`
