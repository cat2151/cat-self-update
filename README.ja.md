# cat-self-update

## 状況

ドッグフーディング中です。

## 用途

Windows版Rust用セルフアップデートのリファレンス実装。

## 方針

シンプルな実装を優先しています。

## 予定
- 以下をライブラリクレートに実装予定です：
  - hash
  - check
  - auto-update
  - back-ground-check
  - force-update（back-ground-checkで更新検知時）
  - アプリ終了時notice（back-ground-checkで更新検知時）

## install

```
cargo install --force --git https://github.com/cat2151/cat-self-update
```

## 実行

```
cat-self-update update
cat-self-update check
```

※Pythonがない場合は正常に動作しません
