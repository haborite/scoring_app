# Scoring App

試験の採点と成績評価を行うデスクトップアプリです。問題と受験者を登録したら、キーボードだけで次々に点数を入力し、重み付きの最終得点と成績区分の分布をその場で確認できます。

[Rust](https://www.rust-lang.org/) と [Dioxus](https://dioxuslabs.com/) (desktop) で作られており、Windows / macOS / Linux で動作します。

## 主な機能

| 画面 | 内容 |
| --- | --- |
| **採点** | 受験者ごとに各問題の点数を入力します。全受験者の点数と最終得点が一覧表で表示されます。問題ごとの採点メモ（コメント）も記録できます。 |
| **問題設定** | 問題名、満点、重み、コメントを登録します。id を変更すると、その問題の入力済みの点数も新しい id に引き継がれます（重複する id は登録できません）。 |
| **受験者設定** | 学籍番号と氏名を登録します。「Add row」を押すと、直前の番号に続く番号で行が追加されます（例: `A009` → `A010`）。学籍番号を変更すると、入力済みの点数も引き継がれます（重複する番号は登録できません）。 |
| **成績評価** | 成績区分（例: 優 / 良 / 可 / 不可）ごとの下限点を設定し、区分ごとの人数と割合、5 点刻みのヒストグラムを表示します。 |

### 最終得点の計算方法

```
最終得点 = Σ (得点 / 満点 × 重み) / Σ 重み × 100
```

最終得点は 0〜100 点（小数点以下は四捨五入）です。未入力の問題が 1 つでもある受験者は計算と集計から除かれます。どの区分の下限点にも届かない受験者は、成績評価画面で「（区分なし）」として数えられます。

## キーボード操作（採点画面）

| キー | テンキー | 動作 |
| --- | --- | --- |
| `Enter` / `↓` | `Enter` / `+` | 次の問題へ |
| `↑` | `-` | 前の問題へ |
| `J` | `/` | 前の受験者へ |
| `L` | `*` | 次の受験者へ |
| `F` | `.` | 受験者検索を開く（学籍番号・氏名で検索） |
| `Esc` | | 検索を閉じる |

検索ウィンドウでは `↑` / `↓` で候補を選び、`Enter` で確定します。

## データの保存

データはすべて 1 つの JSON ファイルに保存されます。画面右上のボタンで操作します。

- **Save**: いま開いているファイルに上書き保存します（まだファイルがなければ Save as と同じ動作になります）
- **Save as**: 保存先を選んで保存します
- **Load**: JSON ファイルを読み込みます

保存していない変更があると、ウィンドウのタイトルに `*` が付きます。その状態でウィンドウを閉じようとすると確認ダイアログが表示され、「保存して終了」「保存せずに終了」「キャンセル（終了しない）」を選べます。

「問題設定」「受験者設定」画面の **Import JSON** を使うと、保存済みのファイルから問題一覧または受験者一覧だけを取り込めます。取り込み用のファイルには `questions` または `students` だけを書いておけば十分です（書かれていない項目は空として扱われます）。

<details>
<summary>JSON の形式</summary>

```json
{
  "questions": [
    { "id": 1, "name": "問1", "full_score": 10, "weight": 1.0, "comment": "" }
  ],
  "students": [
    { "id": "A001", "name": "山田 太郎" }
  ],
  "scores": [
    { "student_id": "A001", "question_id": 1, "score": 8 }
  ],
  "ratings": [
    { "label": "優", "min_score": 80 },
    { "label": "良", "min_score": 70 },
    { "label": "可", "min_score": 60 },
    { "label": "不可", "min_score": 0 }
  ]
}
```

</details>

## インストール

[Releases](https://github.com/haborite/scoring_app/releases) から、お使いの OS 用のファイルをダウンロードしてください。

| OS | ファイル |
| --- | --- |
| Windows (x64) | `*-setup.exe`（NSIS インストーラ）、`*.msi`、`*_portable.zip`（インストール不要。展開して `grading_app.exe` を実行） |
| macOS (Apple Silicon) | `*.dmg`、`*.app.zip` |
| Linux (x64) | `*.deb`、`*.AppImage` |

> [!NOTE]
> 配布物にはコード署名をしていません。
> - **Windows**: SmartScreen の警告が出たら「詳細情報」→「実行」を選んでください。
> - **macOS**: 「開発元を検証できません」と表示されたら、Finder でアプリを右クリックして「開く」を選ぶか、`xattr -cr /Applications/GradingApp.app` を実行してください。
> - **Linux**: AppImage は `chmod +x` で実行権限を付けてから起動してください。WebKitGTK（`libwebkit2gtk-4.1`）が必要です。

## ソースからビルド

### 必要なもの

- [Rust](https://rustup.rs/)（stable）
- [Node.js](https://nodejs.org/)（Tailwind CSS のビルドに使用）
- [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/): `cargo binstall dioxus-cli@0.7.3`（または `cargo install dioxus-cli@0.7.3 --locked`）
- Linux のみ:
  ```sh
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev libxdo-dev librsvg2-dev
  ```

### 開発

```sh
npm ci
npm run css:watch   # assets/tailwind.css を生成し、変更を監視
dx serve            # 別のターミナルで実行
```

`assets/tailwind.css` は自動生成されるファイルなので、Git では管理していません。ビルドの前に必ず `npm run css` か `npm run css:watch` を実行してください。

### 配布物の作成

```sh
npm ci && npm run css
dx bundle --desktop --release --out-dir dist
```

`dist/` に、その OS 用のインストーラが出力されます。

## リリース手順

`v*` 形式のタグを push すると、GitHub Actions（[`.github/workflows/release.yml`](.github/workflows/release.yml)）が Windows / Linux / macOS 用の配布物をビルドし、GitHub Release を作成します。

```sh
# Cargo.toml の version を更新してから
git tag v0.1.0
git push origin v0.1.0
```

Actions タブから手動で実行（workflow_dispatch）することもできます。その場合、リリースは作成されず、ビルドした配布物は Artifacts としてダウンロードできます。

## ライセンス

[MIT License](LICENSE)
