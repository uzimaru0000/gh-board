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
    ShowArchivedList,
    StartFilter,
    ClearFilter,
    Refresh,
    ShowHelp,
    SwitchProject,
    ChangeGrouping,
    ToggleLayout,

    // Detail content
    OpenInBrowser,
    EditCard,
    NewComment,
    OpenCommentList,
    CopyUrl,

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

    // Bulk selection
    BulkSelectStart,
    BulkSelectToggle,
    BulkSelectAll,
    BulkSelectClear,
    BulkArchive,
    BulkMoveLeft,
    BulkMoveRight,
}
