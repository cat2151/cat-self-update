# cat-self-update

## 状況

ドッグフーディング中です。

## 用途

Windows版Rust用セルフアップデートのリファレンス実装。

## 方針

シンプルな実装を優先しています。

## 予定
- 以下をライブラリクレートに実装予定です：
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

## 注意

Pythonがない場合は正常に動作しません

## 運用

- 理想：このライブラリのバグ修正があった場合、このライブラリを利用するアプリはそれを自動的に検知してcargo installできること
- 分析：それを行うには、アプリ側において「このライブラリのためだけに支払うコスト」が大きく、見合わない
- 現実：もし、このライブラリを利用した update サブコマンドそのものが落ちる場合は、cargo install を実行しなおす
