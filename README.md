# Scoring App

試験の採点と成績評価を行うデスクトップアプリです。問題と受験者を登録したら、キーボードだけで次々に点数を入力し、重み付きの最終得点と成績区分の分布をその場で確認できます。

[Rust](https://www.rust-lang.org/) と [Dioxus](https://dioxuslabs.com/) (desktop) で作られており、Windows / macOS / Linux で動作します。

## 主な機能

| 画面 | 内容 |
| --- | --- |
| **採点** | 受験者ごとに各問題の点数を入力します。全受験者の点数と最終得点が一覧表で表示され、一覧表の受験者をクリックするとその受験者の採点に移ります。問題ごとの採点メモ（コメント）も記録できます。満点を超える点数は赤く表示され、保存されません。コメント・点数入力・一覧表の 3 つの区画は、区切り線をドラッグして大きさを変えられます。 |
| **問題設定** | 問題名、満点、重み、コメントを登録します。id を変更すると、その問題の入力済みの点数も新しい id に引き継がれます（重複する id は登録できません）。「Add row」で追加した問題の満点は、設定画面で決めた初期値（最初は 8）になります。 |
| **受験者設定** | 受験者IDと氏名を登録します。CSV ファイルから読み込むこともできます。「Add row」を押すと、直前の番号に続く番号で行が追加されます（例: `A009` → `A010`）。受験者IDを変更すると、入力済みの点数も引き継がれます（重複する番号は登録できません）。 |
| **成績評価** | 成績区分（例: 優 / 良 / 可 / 不可）ごとの下限点を設定し、区分ごとの人数と割合、5 点刻みのヒストグラムを表示します。 |
| **設定** | 新しい問題の満点の初期値、採点画面のレイアウト、キー割り当てを変更します。設定は自動で保存され、どの採点ファイルでも共通です。 |

### 最終得点の計算方法

```
最終得点 = Σ (得点 / 満点 × 重み) / Σ 重み × 100
```

最終得点は 0〜100 点（小数点以下は四捨五入）です。未入力の問題が 1 つでもある受験者は計算と集計から除かれます。要再採点の点数がある受験者も集計から除かれます（下記）。どの区分の下限点にも届かない受験者は、成績評価画面で「（区分なし）」として数えられます。

### 要再採点

点数を入力したあとでその問題の満点を変更すると、その点数は「要再採点」になり、採点画面の一覧表と受験者欄に ⚠ が表示されます（例: 「満点 5→8」）。満点を採点時の値に戻すと、表示は消えます。

- 点数を入力し直すと、その時点の満点で採点したものとして扱われます。
- 点数を変えなくてよい場合は、その問題の「確認済み」ボタン（または点数入力欄で `Ctrl+Enter`）を押します。満点を超える点数は確認済みにできません。
- 要再採点の点数がある受験者は、Completed の人数と成績評価の集計に含まれません。
- `Ctrl+Shift+L` で、次の要再採点の受験者に移動できます。

## キーボード操作

キー割り当ては「設定」画面で変更できます。以下は初期値です。`Ctrl` を使うショートカットは、どの入力欄にカーソルがあっても使えます（macOS では `Ctrl` の代わりに `⌘`）。

| キー | 動作 |
| --- | --- |
| `Ctrl+S` | 上書き保存 |
| `Ctrl+Shift+S` | 名前を付けて保存 |
| `Ctrl+O` | ファイルを開く |
| `Ctrl+1` 〜 `Ctrl+5` | 問題設定 / 受験者設定 / 採点 / 成績評価 / 設定 画面へ |
| `Ctrl+F` | 受験者検索を開く（受験者ID・氏名で検索） |
| `Ctrl+J` / `Ctrl+L` | 前の受験者 / 次の受験者（採点画面） |
| `Ctrl+Shift+L` | 次の要再採点の受験者へ |
| `Ctrl+E` | 点数入力欄とコメント欄を行き来する（採点画面。コメント欄では `Esc` でも戻れます） |

採点画面の点数入力欄では、次のキーも使えます。

| キー | テンキー | 動作 |
| --- | --- | --- |
| `Enter` / `↓` | `Enter` / `+` | 次の問題へ |
| `↑` | `-` | 前の問題へ |
| | `/` / `*` | 前の受験者 / 次の受験者 |
| | `.` | 受験者検索を開く |
| `Ctrl+Enter` | | 要再採点の点数を確認済みにする |

検索ウィンドウでは `↑` / `↓` で候補を選び、`Enter` で確定、`Esc` で閉じます。採点画面の下の一覧表で受験者IDか氏名をクリックすると、その受験者の採点に移ります。

### キー割り当ての変更

「設定」画面のキー割り当てで「追加」を押し、割り当てたいキーを押します（`Esc` で取り消し）。1 つの操作に複数のキーを割り当てられます。

- どこでも使える操作には `Ctrl`（`⌘`）を含むキーが必要です。
- `Ctrl+C` / `V` / `X` / `Z` / `Y` / `A` は文字の編集に使うため割り当てられません。点数入力欄の操作には、数字・BackSpace・Delete・Tab・`←` / `→` などの入力用のキーは割り当てられません。
- ほかの操作で使っているキーは割り当てられません（先にそちらから外してください）。

## アプリの設定

「設定」画面の内容は、採点ファイルとは別の `settings.json` に自動で保存され、どの採点ファイルを開いても同じ設定が使われます。設定を変えても、採点ファイルが未保存の状態にはなりません。

| OS | 場所 |
| --- | --- |
| Windows | `%APPDATA%\scoring_app\settings.json` |
| macOS | `~/Library/Application Support/scoring_app/settings.json` |
| Linux | `~/.config/scoring_app/settings.json` |

## データの保存

データはすべて 1 つの JSON ファイルに保存されます。画面右上のボタンで操作します。

- **Save**: いま開いているファイルに上書き保存します（まだファイルがなければ Save as と同じ動作になります）
- **Save as**: 保存先を選んで保存します
- **Load**: JSON ファイルを読み込みます（保存していない変更があるときは確認が表示されます）

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
    { "student_id": "A001", "question_id": 1, "score": 8, "graded_full_score": 10 }
  ],
  "ratings": [
    { "label": "優", "min_score": 80 },
    { "label": "良", "min_score": 70 },
    { "label": "可", "min_score": 60 },
    { "label": "不可", "min_score": 0 }
  ]
}
```

`graded_full_score` は、その点数を入力した（または確認済みにした）ときの満点です。要再採点の判定に使います。これがない点数（以前のバージョンで保存したファイルなど）は、満点を超えていない限り要再採点になりません。

</details>

## CSV から受験者を読み込む

「受験者設定」画面で CSV フォーマットを選び、**Import CSV** を押してファイルを選ぶと、受験者一覧がその内容に置き換わります。点数は受験者IDで結び付いているので、同じ受験者IDの受験者の点数はそのまま残ります。

組み込みのフォーマットは次の 2 つです。

| フォーマット | 対象のファイル | 受験者ID / 氏名の列 |
| --- | --- | --- |
| UTAS | UTAS の履修者名簿（4 行目が見出し行） | `学生証番号` / `氏名` |
| UTOL 課題 | UTOL の課題提出状況 CSV（1 行目が見出し行） | `[#]学生証番号` / `氏名` |

- 受験者IDが空の行と、すでに読み込んだ受験者IDと重複する行は読み飛ばされ、その行番号が画面に表示されます。
- 受験者IDの先頭の `0` は消えません（`01234567` は `01234567` のまま読み込まれます）。

### フォーマットを追加する

**CSV フォーマット定義...** ボタンを押すと、定義ファイル `csv_formats.json` が入っているフォルダが開きます。このファイルがまだなければ、例を 1 つ書き込んだ状態で作成されます。

| OS | 場所 |
| --- | --- |
| Windows | `%APPDATA%\scoring_app\csv_formats.json` |
| macOS | `~/Library/Application Support/scoring_app/csv_formats.json` |
| Linux | `~/.config/scoring_app/csv_formats.json` |

ファイルの中身はフォーマットの配列です。ファイルはフォーマットの選択欄を開くたびに読み直されるので、編集後にアプリを再起動する必要はありません。組み込みのフォーマットと同じ `name` で定義すると、組み込みのフォーマットを上書きできます。

```json
[
  {
    "name": "Excel 名簿",
    "description": "1 行目が見出し、Shift_JIS",
    "encoding": "shift_jis",
    "id_header": "受験者ID",
    "name_header": "氏名"
  }
]
```

| 項目 | 既定値 | 内容 |
| --- | --- | --- |
| `name` | （必須） | 選択欄に表示される名前 |
| `description` | `""` | 説明（選択欄にマウスを重ねると表示されます） |
| `encoding` | `"auto"` | `"auto"`（UTF-8 として読めなければ Shift_JIS）、`"utf-8"`、`"shift_jis"` |
| `delimiter` | `","` | 区切り文字（半角 1 文字。タブ区切りなら `"\t"`） |
| `quote` | `"\""` | 囲み文字。`null` にすると囲み文字を使いません |
| `direction` | `"rows"` | `"rows"`: 1 行に 1 人。`"columns"`: 1 列に 1 人（見出しが縦に並ぶ表） |
| `skip_before_header` | `0` | 見出し行の前に読み飛ばす行数（`columns` の場合は列数）。空行は数えません |
| `skip_after_header` | `0` | 見出し行と最初の受験者の間で読み飛ばす行数 |
| `id_header` | （必須） | 受験者IDの列の見出し |
| `name_header` | （必須） | 氏名の列の見出し |
| `trim_whitespace` | `true` | 各セルの前後の空白（全角スペースを含む）を取り除くか |

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
