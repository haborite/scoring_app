# 開発者向け情報

Scoring App のビルド・開発環境・リリース手順です。利用者向けの使い方は [README](../README.md) を、CSV フォーマットの仕様は [formats/README.md](../formats/README.md) を参照してください。

## 実装言語・主なライブラリ

- 言語: [Rust](https://www.rust-lang.org/)（edition 2021）
- UI フレームワーク: [Dioxus](https://dioxuslabs.com/) 0.7（`desktop` フィーチャー。WebView 上に Rust で書いた UI を表示するデスクトップアプリ）
- スタイル: [Tailwind CSS](https://tailwindcss.com/) v4 + [daisyUI](https://daisyui.com/) v5（`npm` でビルドし、`assets/tailwind.css` として同梱）

### 依存クレート（[`Cargo.toml`](../Cargo.toml)）

| クレート | 用途 |
| --- | --- |
| `dioxus`, `dioxus-desktop` | UI とデスクトップウィンドウ |
| `rfd` | ファイル選択・保存ダイアログ、確認ダイアログ（Linux は gtk3 バックエンド） |
| `tokio` | 非同期ランタイム（ファイル I/O、ダイアログの待機など） |
| `serde`, `serde_json` | 保存ファイル・設定・CSV フォーマット定義の JSON シリアライズ |
| `csv` | CSV の読み書き |
| `encoding_rs` | Shift_JIS ⇔ UTF-8 の変換 |
| `dirs` | OS ごとの設定フォルダの取得 |
| `anyhow`, `thiserror` | エラー型 |

### npm 依存（[`package.json`](../package.json)）

`tailwindcss` と `daisyui`（CLI でビルドするだけで、アプリ自体には Node.js は不要）。

## ディレクトリ構成

```
src/
  main.rs            アプリのエントリポイント、キーボードショートカットの配線
  models.rs          保存データ（問題・受験者・点数・成績区分）とその操作
  csv_import.rs       受験者 CSV インポートの実装とフォーマット定義の読み込み
  csv_export.rs       成績 CSV エクスポートの実装とフォーマット定義の読み込み
  report.rs          最終得点・成績区分・分布の集計
  settings.rs        アプリ設定（settings.json）
  shortcuts.rs       キー割り当て
  ui/                画面ごとのコンポーネント
formats/
  import_formats.json   受験者インポートの付属フォーマット（ビルド時に埋め込み）
  export_formats.json   成績エクスポートの付属フォーマット（ビルド時に埋め込み）
  README.md             フォーマットの仕様（→ フォーマットを追加するときはここを参照）
reference_files/     CSV インポートのテストで使うサンプルファイル
docs/
  DEVELOPMENT.md     このファイル
```

付属の CSV フォーマットは Rust のコードには直書きされておらず、`formats/*.json` が `include_str!` でバイナリに埋め込まれ、実行時に利用者の設定フォルダへ書き出されます。フォーマットの追加・変更は [formats/README.md](../formats/README.md) を参照してください。

## 開発環境の構築

### 必要なもの

- [Rust](https://rustup.rs/)（stable）
- [Node.js](https://nodejs.org/)（Tailwind CSS のビルドに使用。アプリの実行自体には不要）
- [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/): `cargo binstall dioxus-cli@0.7.3`（または `cargo install dioxus-cli@0.7.3 --locked`）
- Linux のみ:
  ```sh
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev libxdo-dev librsvg2-dev
  ```

### 起動

```sh
npm ci
npm run css:watch   # assets/tailwind.css を生成し、変更を監視
dx serve            # 別のターミナルで実行
```

`assets/tailwind.css` は自動生成されるファイルなので、Git では管理していません。ビルドの前に必ず `npm run css` か `npm run css:watch` を実行してください。

### テスト

```sh
cargo test
```

付属の CSV フォーマット（`formats/*.json`）も、このテストの中で実際に読み書きして検証しています。フォーマットの JSON が壊れている、または仕様に合わない内容だとテストが失敗します。

### 配布物の作成

```sh
npm ci && npm run css
dx bundle --desktop --release --out-dir dist
```

`dist/` に、その OS 用のインストーラが出力されます。

## リリース手順

`v*` 形式のタグを push すると、GitHub Actions（[`.github/workflows/release.yml`](../.github/workflows/release.yml)）が Windows / Linux / macOS 用の配布物をビルドし、GitHub Release を作成します。

```sh
# Cargo.toml の version を更新してから
git tag v0.1.0
git push origin v0.1.0
```

Actions タブから手動で実行（`workflow_dispatch`）することもできます。その場合、リリースは作成されず、ビルドした配布物は Artifacts としてダウンロードできます。
