use super::*;

impl AppState {
    pub(super) fn switch_to_view(&mut self, idx: usize) -> Command {
        if idx >= self.views.len() {
            return Command::None;
        }
        self.active_view = Some(idx);
        let filter_str = self.resolve_at_me(&self.views[idx].filter.clone());
        self.filter.input = filter_str.clone();
        self.filter.cursor_pos = filter_str.len();
        if filter_str.is_empty() {
            self.filter.active_filter = None;
        } else {
            self.filter.active_filter = Some(ActiveFilter::parse(&filter_str));
        }
        self.selected_card = 0;
        self.scroll_offset = 0;
        self.current_layout = match self.views[idx].layout {
            Some(LayoutModeConfig::Table) => LayoutMode::Table,
            Some(LayoutModeConfig::Roadmap) => LayoutMode::Roadmap,
            _ => LayoutMode::Board,
        };
        self.table_selected_row = 0;
        self.roadmap_selected_row = 0;
        if let Some(project) = &self.current_project {
            let id = project.id.clone();
            self.start_loading_board(&id)
        } else {
            Command::None
        }
    }

    pub(super) fn clear_view(&mut self) -> Command {
        self.active_view = None;
        self.filter.active_filter = None;
        self.filter.input.clear();
        self.filter.cursor_pos = 0;
        self.selected_card = 0;
        self.scroll_offset = 0;
        self.current_layout = LayoutMode::Board;
        self.table_selected_row = 0;
        self.roadmap_selected_row = 0;
        if let Some(project) = &self.current_project {
            let id = project.id.clone();
            self.start_loading_board(&id)
        } else {
            Command::None
        }
    }

    /// ProjectSelect モードに遷移する。projects 未ロードなら取得を開始する。
    pub fn enter_project_select(&mut self) -> Command {
        self.mode = ViewMode::ProjectSelect;
        if self.projects.is_empty() {
            self.start_loading_projects()
        } else {
            Command::None
        }
    }

    pub fn select_project(&mut self, index: usize) -> Command {
        if let Some(project) = self.projects.get(index) {
            let project = project.clone();
            self.current_project = Some(project.clone());
            self.project_filter_query.clear();
            self.project_filter_cursor = 0;
            self.recompute_filtered_projects();
            // モーダルを閉じ、Board 画面を Loading 表示にする。
            // 別プロジェクトの board/キャッシュは持ち越さない。
            self.mode = ViewMode::Board;
            self.board = None;
            self.invalidate_board_cache();
            self.start_loading_board(&project.id)
        } else {
            Command::None
        }
    }

    pub fn real_project_index(&self) -> Option<usize> {
        self.filtered_project_indices
            .get(self.selected_project_index)
            .copied()
    }

    pub fn recompute_filtered_projects(&mut self) {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;

        if self.project_filter_query.is_empty() {
            self.filtered_project_indices = (0..self.projects.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let pattern = &self.project_filter_query;
            let mut scored: Vec<(i64, usize, usize)> = self
                .projects
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    let haystack = match &p.description {
                        Some(d) if !d.is_empty() => format!("{} {}", p.title, d),
                        _ => p.title.clone(),
                    };
                    matcher
                        .fuzzy_match(&haystack, pattern)
                        .map(|score| (score, i, i))
                })
                .collect();
            // スコア降順、同点は元順 (tie-breaker に元 index の昇順)
            scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
            self.filtered_project_indices = scored.into_iter().map(|(_, i, _)| i).collect();
        }
        self.selected_project_index = 0;
    }

    pub(super) fn handle_project_select_key(&mut self, key: KeyEvent) -> Command {
        // 1. 構造キー優先 (Esc/Enter/Down/Up/ForceQuit)
        if let Some(action) = self.keymap.resolve(KeymapMode::ProjectSelect, &key) {
            match action {
                Action::ForceQuit => {
                    self.should_quit = true;
                    return Command::None;
                }
                Action::Quit => {
                    if !self.project_filter_query.is_empty() {
                        self.project_filter_query.clear();
                        self.project_filter_cursor = 0;
                        self.recompute_filtered_projects();
                    } else if self.board.is_some() {
                        self.mode = ViewMode::Board;
                    } else {
                        self.should_quit = true;
                    }
                    return Command::None;
                }
                Action::MoveDown => {
                    if !self.filtered_project_indices.is_empty() {
                        self.selected_project_index = (self.selected_project_index + 1)
                            .min(self.filtered_project_indices.len() - 1);
                    }
                    return Command::None;
                }
                Action::MoveUp => {
                    self.selected_project_index = self.selected_project_index.saturating_sub(1);
                    return Command::None;
                }
                Action::Select => {
                    return match self.real_project_index() {
                        Some(idx) => self.select_project(idx),
                        None => Command::None,
                    };
                }
                _ => {}
            }
        }

        // 2. テキスト入力 (常時入力可)
        match key.code {
            KeyCode::Backspace if self.project_filter_cursor > 0 => {
                let new_len = self
                    .project_filter_query
                    .char_indices()
                    .take_while(|(i, _)| *i < self.project_filter_cursor)
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                self.project_filter_query.truncate(new_len);
                self.project_filter_cursor = new_len;
                self.recompute_filtered_projects();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.project_filter_query
                    .insert(self.project_filter_cursor, c);
                self.project_filter_cursor += c.len_utf8();
                self.recompute_filtered_projects();
            }
            _ => {}
        }
        Command::None
    }

    pub(super) fn handle_filter_key(&mut self, key: KeyEvent) -> Command {
        // Check structural keys first (ForceQuit, Back, Select)
        if let Some(action) = self.keymap.resolve(KeymapMode::FilterStructural, &key) {
            match action {
                Action::ForceQuit => {
                    self.should_quit = true;
                    return Command::None;
                }
                Action::Back => {
                    self.mode = ViewMode::Board;
                    return Command::None;
                }
                Action::Select => {
                    self.active_view = None;
                    let resolved = self.resolve_at_me(&self.filter.input.clone());
                    if resolved.is_empty() {
                        self.filter.active_filter = None;
                    } else {
                        self.filter.active_filter = Some(ActiveFilter::parse(&resolved));
                    }
                    self.selected_card = 0;
                    self.scroll_offset = 0;
                    self.mode = ViewMode::Board;
                    return if let Some(project) = &self.current_project {
                        let id = project.id.clone();
                        self.start_loading_board(&id)
                    } else {
                        Command::None
                    };
                }
                _ => {}
            }
        }

        // Text input handling (not configurable)
        match key.code {
            KeyCode::Backspace if self.filter.cursor_pos > 0 => {
                let prev = prev_char_pos(&self.filter.input, self.filter.cursor_pos);
                self.filter.input.drain(prev..self.filter.cursor_pos);
                self.filter.cursor_pos = prev;
            }
            KeyCode::Left if self.filter.cursor_pos > 0 => {
                self.filter.cursor_pos =
                    prev_char_pos(&self.filter.input, self.filter.cursor_pos);
            }
            KeyCode::Right if self.filter.cursor_pos < self.filter.input.len() => {
                self.filter.cursor_pos =
                    next_char_pos(&self.filter.input, self.filter.cursor_pos);
            }
            KeyCode::Char(c) => {
                self.filter.input.insert(self.filter.cursor_pos, c);
                self.filter.cursor_pos += c.len_utf8();
            }
            _ => {}
        }
        Command::None
    }
}
