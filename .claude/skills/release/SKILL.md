---
name: release
description: 変更内容からセマンティックバージョニングに基づいてバージョンを判定し、リリース準備を行う
allowed-tools: Bash Read Edit Grep
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

### 4. リリース準備を実行

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
