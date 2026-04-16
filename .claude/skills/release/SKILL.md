---
name: release
description: 変更内容からセマンティックバージョニングに基づいてバージョンを判定し、リリース準備を行う
allowed-tools: Bash Read Edit Grep Write
argument-hint: "[patch|minor|major]"
---

# リリース準備

前回のリリース以降の変更内容を分析し、適切なバージョンを決定してリリースを準備する。

## 手順

### 1. 現在の状態を確認

- `Cargo.toml` から現在のバージョンを取得する
- 最新の git タグ (`v*`) を取得する。タグがなければ初回リリースとして扱う
- **未コミットの変更がないことを確認する。**ある場合はユーザーに警告して中断する

### 2. 変更内容を分析

前回のタグ (なければ初回コミット) から HEAD までのコミットログを取得し、変更の種類を分類する:

- **BREAKING CHANGE** (API の互換性を壊す変更、既存機能の削除) → **major**
- **新機能追加** (feat, add) → **minor**
- **バグ修正、リファクタ、ドキュメント、CI 変更** (fix, refactor, docs, ci, chore) → **patch**

### 3. バージョンを決定

- 引数 `$ARGUMENTS` が `patch` / `minor` / `major` のいずれかなら、それを採用する
- 引数がなければ、変更内容から自動判定した結果をユーザーに提案し、確認を取る
- セマンティックバージョニング: `major.minor.patch`

### 4. CHANGELOG を更新

`CHANGELOG.md` に新バージョンのエントリを追加する。

1. `CHANGELOG.md` を読み込む
2. ステップ 2 で分析したコミットを [Keep a Changelog](https://keepachangelog.com/) 形式で分類する:
   - **Added** — 新機能 (feat, add)
   - **Changed** — 既存機能の変更
   - **Fixed** — バグ修正 (fix)
   - **Removed** — 削除された機能
   - **Deprecated** — 非推奨になった機能
   - Merge コミット、chore: bump version、CI のみの変更はスキップする
3. `## [Unreleased]` の直後 (なければ最初の `## [x.y.z]` の直前) に新しいセクションを挿入する:
   ```markdown
   ## [X.Y.Z] - YYYY-MM-DD

   ### Added
   - 変更内容 (#Issue番号)
   ```
4. ファイル末尾のリンク一覧にも新バージョンのリンクを追加する:
   ```markdown
   [X.Y.Z]: https://github.com/uzimaru0000/gh-board/releases/tag/vX.Y.Z
   ```
5. ユーザーに CHANGELOG の内容を見せて確認を取る。修正の要望があれば反映する

### 5. リリース準備を実行

確認が取れたら、以下を行う:

1. `Cargo.toml` の `version` を新バージョンに更新する
2. `cargo check` を実行してビルドが通ることを確認する (schema.graphql がなければ `build.rs` が自動ダウンロードする)
3. 変更をコミットする: `chore: bump version to vX.Y.Z`
4. git タグを作成する: `vX.Y.Z`
5. **push はしない。** ユーザーに以下のコマンドを案内する:
   ```
   git push origin main --follow-tags
   ```

### 注意事項

- push やリリース作成は絶対に自動実行しない。コマンドの案内のみ行う
- タグの push により `.github/workflows/release.yml` が起動し、クロスコンパイル → GitHub Release 作成が自動で行われる
- CHANGELOG は人間が読むものなので、コミットメッセージをそのまま貼るのではなく、ユーザー視点で意味のある変更内容にまとめる
