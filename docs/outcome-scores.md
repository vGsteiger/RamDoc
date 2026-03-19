# Outcome Scores Feature

This document describes the outcome scores feature implementation for standardized mental health questionnaires.

## Overview

The outcome scores feature allows clinicians to administer, score, and track standardized outcome questionnaires (PHQ-9, GAD-7, BDI-II) within patient sessions.

## Supported Scales

### PHQ-9 (Patient Health Questionnaire-9)
- **Purpose**: Depression screening
- **Score Range**: 0-27
- **Interpretation**:
  - 0-4: Minimal
  - 5-9: Mild
  - 10-14: Moderate
  - 15-19: Moderately Severe
  - 20-27: Severe

### GAD-7 (Generalized Anxiety Disorder-7)
- **Purpose**: Anxiety screening
- **Score Range**: 0-21
- **Interpretation**:
  - 0-4: Minimal
  - 5-9: Mild
  - 10-14: Moderate
  - 15-21: Severe

### BDI-II (Beck Depression Inventory-II)
- **Purpose**: Depression assessment
- **Score Range**: 0-63
- **Interpretation**:
  - 0-13: Minimal
  - 14-19: Mild
  - 20-28: Moderate
  - 29-63: Severe

## Database Schema

### outcome_scores table

```sql
CREATE TABLE outcome_scores (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    scale_type TEXT NOT NULL,  -- 'PHQ-9', 'GAD-7', 'BDI-II'
    score INTEGER NOT NULL,
    interpretation TEXT,  -- Auto-calculated based on score
    subscores TEXT,  -- JSON for item-level scores
    administered_at TEXT NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
```

## API Endpoints

### Backend Commands (Rust/Tauri)

- `create_outcome_score(input: CreateOutcomeScore) -> OutcomeScore`
- `get_outcome_score(id: String) -> OutcomeScore`
- `list_scores_for_session(session_id: String, limit?: u32, offset?: u32) -> Vec<OutcomeScore>`
- `list_scores_by_scale(scale_type: String, limit?: u32, offset?: u32) -> Vec<OutcomeScore>`
- `update_outcome_score(id: String, input: UpdateOutcomeScore) -> OutcomeScore`
- `delete_outcome_score(id: String) -> ()`

### Frontend API (TypeScript)

```typescript
// Types
interface OutcomeScore {
  id: string;
  session_id: string;
  scale_type: string;
  score: number;
  interpretation: string | null;
  subscores: string | null;
  administered_at: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

// Functions
createOutcomeScore(input: CreateOutcomeScore): Promise<OutcomeScore>
getOutcomeScore(id: string): Promise<OutcomeScore>
listScoresForSession(sessionId: string, limit?: number, offset?: number): Promise<OutcomeScore[]>
updateOutcomeScore(id: string, input: UpdateOutcomeScore): Promise<OutcomeScore>
deleteOutcomeScore(id: string): Promise<void>
```

## Components

### OutcomeScoreForm
A form component for creating or editing outcome scores.

**Props:**
- `outcomeScore?: OutcomeScore` - Optional existing score to edit
- `sessionId?: string` - Session ID for new scores
- `onSave: (input) => void` - Save callback
- `onCancel: () => void` - Cancel callback

### OutcomeScoreCard
A display card component for showing outcome score information.

**Props:**
- `outcomeScore: OutcomeScore` - The score to display
- `onEdit?: () => void` - Optional edit callback
- `onDelete?: () => void` - Optional delete callback

## Usage

### Viewing Scores in a Session

1. Navigate to a patient's sessions list
2. Click on a specific session
3. The session detail page shows all outcome scores for that session
4. Scores are displayed with color-coded severity badges

### Adding a New Score

1. On the session detail page, click "Neuer Score"
2. Select the questionnaire type (PHQ-9, GAD-7, or BDI-II)
3. Enter the total score
4. Add any relevant notes
5. Click "Hinzufügen" to save

### Editing a Score

1. Click the edit icon on any outcome score card
2. Modify the fields as needed
3. Click "Aktualisieren" to save changes

### Deleting a Score

1. Click the delete icon on any outcome score card
2. Confirm the deletion in the dialog

## Validation

The backend automatically validates:
- Scale type (must be PHQ-9, GAD-7, or BDI-II)
- Score ranges (must be within valid range for each scale)
- Interpretation is auto-calculated based on score and scale type

## Search and Indexing

Outcome scores are automatically indexed in the full-text search system. Search queries can find scores by:
- Scale type
- Interpretation level
- Patient name
- Date administered
- Notes content

## Translations

The feature is fully translated in German and English:
- Form labels and buttons
- Error messages
- Scale names and interpretations

## Future Enhancements

Potential improvements for future versions:
- Item-level scoring support (subscores field is prepared)
- Trend visualization across multiple sessions
- Automatic reminders for periodic reassessment
- Export scores to reports
- Comparison with normative data
