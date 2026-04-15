#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Universal
    Quit,
    ForceQuit,
    Back,

    // Navigation
    MoveDown,
    MoveUp,
    MoveLeft,
    MoveRight,
    FirstItem,
    LastItem,
    NextTab,
    PrevTab,

    // Board
    OpenDetail,
    GrabCard,
    NewCard,
    ArchiveCard,
    UnarchiveCard,
    ShowArchivedList,
    StartFilter,
    ClearFilter,
    Refresh,
    ShowHelp,
    SwitchProject,
    ChangeGrouping,

    // Detail content
    OpenInBrowser,
    EditCard,
    NewComment,
    OpenCommentList,

    // Detail sidebar / confirm / grab / forms
    Select,
    ConfirmYes,
    ConfirmNo,
    ConfirmGrab,
    CancelGrab,
    EditComment,
    Submit,
    NextField,
    PrevField,
    ToggleType,
    OpenEditor,
    ToggleItem,

    // Reactions
    OpenReactionPicker,
    ToggleReaction,
}
