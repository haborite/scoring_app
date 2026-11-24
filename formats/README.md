# CSV フォーマット

このフォルダの `import_formats.json` / `export_formats.json` は、Scoring App に付属している CSV フォーマットの定義です。Rust のコードには直書きされておらず、ビルド時にバイナリへ埋め込まれて、実行時に利用者の設定フォルダへそのまま書き出されます。

| フォーマット | 用途 | 書き出し先（初回起動時に作成） |
| --- | --- | --- |
| [`import_formats.json`](import_formats.json) | 受験者一覧の CSV インポート | `csv_formats.json` |
| [`export_formats.json`](export_formats.json) | 成績の CSV エクスポート | `csv_export_formats.json` |

| OS | 設定フォルダ |
| --- | --- |
| Windows | `%APPDATA%\scoring_app\` |
| macOS | `~/Library/Application Support/scoring_app/` |
| Linux | `~/.config/scoring_app/` |

書き出された先のファイルが、そのときアプリが使うフォーマットの一覧そのものです。付属のフォーマットもここに書き出されているので、利用者はそのまま編集・削除・追加できます（組み込みと上書きを区別する仕組みはありません）。ファイルを削除すると、次にフォーマットを読むときにこの `formats/` の内容から作り直されます。同じフォルダに作られる `csv_formats.default.json` / `csv_export_formats.default.json` は、アプリ更新時にこのリポジトリの内容で毎回上書きされる控えです。アプリ内でのボタンの操作は本体の [README](../README.md) を参照してください。

このドキュメントは、新しいフォーマットを追加したい開発者・利用者のための仕様です。

## 受験者インポート（`import_formats.json`）

1 つの要素が 1 つの CSV フォーマットです。

```json
{
  "name": "Excel 名簿",
  "description": "1 行目が見出し、Shift_JIS",
  "encoding": "shift_jis",
  "delimiter": ",",
  "quote": "\"",
  "direction": "rows",
  "skip_before_header": 0,
  "skip_after_header": 0,
  "id_header": "受験者ID",
  "name_header": "氏名",
  "name_separator": " ",
  "trim_whitespace": true
}
```

| 項目 | 既定値 | 内容 |
| --- | --- | --- |
| `name` | （必須） | 選択欄に表示される名前。重複しないようにしてください |
| `description` | `""` | 説明（選択欄にマウスを重ねると表示されます） |
| `encoding` | `"auto"` | `"auto"`（UTF-8 として読めなければ Shift_JIS）、`"utf-8"`、`"shift_jis"` |
| `delimiter` | `","` | 区切り文字（半角 1 文字。タブ区切りなら `"\t"`） |
| `quote` | `"\""` | 囲み文字。`null` にすると囲み文字を使いません |
| `direction` | `"rows"` | `"rows"`: 1 行に 1 人。`"columns"`: 1 列に 1 人（見出しが縦に並ぶ表） |
| `skip_before_header` | `0` | 見出し行の前に読み飛ばす行数（`columns` の場合は列数）。空行は数えません |
| `skip_after_header` | `0` | 見出し行と最初の受験者の間で読み飛ばす行数 |
| `id_header` | （必須） | 受験者IDの列の見出し |
| `name_header` | （必須） | 氏名の列の見出し。文字列 1 つ、または `["姓", "名"]` のように配列で並べると、その順に連結します |
| `name_separator` | `" "` | `name_header` を複数指定したときに、間に入れる文字列 |
| `trim_whitespace` | `true` | 各セルの前後の空白（全角スペースを含む）を取り除くか |

読み込みの挙動:

- 受験者IDが空の行と、すでに読み込んだ受験者IDと重複する行は読み飛ばされ、その行番号が画面に表示されます。
- 受験者IDの先頭の `0` は消えません（`01234567` は `01234567` のまま読み込まれます）。
- `id_header` / `name_header` に指定した見出しが見つからないときは、実際の見出し行の内容がエラーメッセージに表示されます。

### 付属フォーマット

| フォーマット | 対象のファイル | 受験者ID / 氏名の列 |
| --- | --- | --- |
| UTAS | 東京大学 UTAS の履修者名簿（4 行目が見出し行） | `学生証番号` / `氏名` |
| UTOL 課題 | 東京大学 UTOL の課題提出状況 CSV | `[#]学生証番号` / `氏名` |
| 汎用（学籍番号・氏名） | 1 行目が見出しの CSV（Excel で作った名簿など） | `学籍番号` / `氏名` |
| 汎用（student_id・name） | 同じく 1 行目が見出しの CSV | `student_id` / `name` |
| Moodle 成績表 | Moodle の「成績 > エクスポート > プレーンテキストファイル」 | `ID number` / `Surname` + `First name` |
| Moodle 成績表（日本語） | 同じものの日本語表示 | `ID番号` / `姓` + `名` |
| Moodle 評定ワークシート | Moodle の課題「評定ワークシートをダウンロード」 | `Identifier` / `Full name` |
| Canvas 成績表 | Canvas LMS の成績エクスポート CSV | `SIS User ID` / `Student` |
| Blackboard 成績センター | Blackboard Learn の成績表ダウンロード（カンマ区切り） | `Student ID` / `Last Name` + `First Name` |
| WebClass 成績一覧 | WebClass の成績一覧ダウンロード CSV | `ユーザID` / `氏名` |
| OneRoster users.csv | 1EdTech OneRoster v1.1 CSV（教育データ連携の標準形式） | `sourcedId` / `familyName` + `givenName` |

見出しの名前はシステムの表示言語やバージョンによって変わることがあるので、合わないときは利用者の設定フォルダにある定義ファイルを直接編集してください。

- Moodle 評定ワークシートの受験者IDは参加者番号（`Identifier`）です。同じワークシートに書き戻す（エクスポート側の「Moodle 評定ワークシート」）ときはこのまま読み込んでください。
- OneRoster の `users.csv` には教員などの行も含まれます。不要な行は読み込んだあとに「受験者設定」画面で削除してください。

## 成績エクスポート（`export_formats.json`）

1 つの要素が 1 つの CSV フォーマットです。方式（`mode`）によって動作が異なります。

| 方式 | 動作 |
| --- | --- |
| `"new"`（新規作成） | 指定した列だけの CSV を新しく作ります。受験者一覧の全員が 1 行ずつ並びます。 |
| `"fill"`（既存ファイルに記入） | 他のシステムからダウンロードした CSV を選ぶと、受験者IDで照合して指定した列だけを書き込み、別名で保存します。それ以外の行・列・文字コード・改行はそのまま残ります。 |

```json
{
  "name": "成績一覧（Shift_JIS）",
  "description": "",
  "mode": "new",
  "read_as": "",
  "encoding": "shift_jis",
  "delimiter": ",",
  "line_ending": "crlf",
  "columns": [
    { "header": "学生証番号", "value": "id" },
    { "header": "氏名", "value": "name" },
    { "header": "評語", "value": "rating" },
    { "header": "合否", "value": "rating", "map": { "不可": "否", "*": "合" } }
  ]
}
```

```json
{
  "name": "UTAS（評語と合否）",
  "mode": "fill",
  "read_as": "UTAS",
  "columns": [
    { "header": "評語", "value": "rating" },
    { "header": "合否", "value": "rating", "map": { "不可": "否", "*": "合" } }
  ]
}
```

| 項目 | 既定値 | 内容 |
| --- | --- | --- |
| `name` | （必須） | 選択欄に表示される名前。重複しないようにしてください |
| `description` | `""` | 説明（選択欄にマウスを重ねると表示されます） |
| `mode` | `"new"` | `"new"`: 新規作成（**CSV を作成...**）、`"fill"`: 既存ファイルに記入（**CSV に記入...**） |
| `read_as` | | `fill` のとき必須。書き込む CSV を読む、受験者インポート用フォーマットの名前（見出し行の位置と受験者ID列をこれで決めます） |
| `encoding` | `"utf-8-bom"` | `new` の文字コード。`"utf-8"`、`"utf-8-bom"`、`"shift_jis"`（`fill` は元のファイルと同じになります） |
| `delimiter` | `","` | `new` の区切り文字 |
| `line_ending` | `"crlf"` | `new` の改行。`"crlf"` または `"lf"` |
| `columns` | （必須） | 列の配列（下記） |

`columns` の各要素:

| 項目 | 内容 |
| --- | --- |
| `header` | `new`: 書き出す列の見出し。`fill`: 書き込む列の見出し（元のファイルにある見出し） |
| `value` | `id`、`name`、`score`（最終得点）、`rating`（区分）、`q:<問題id>`（その問題の点数）、`literal:<文字列>`（固定の文字列） |
| `map` | 値の置き換え。まず値と同じキーを探し、なければ `"*"`（空でない値すべて）を使います。空欄を置き換えるときは `""` をキーにします |

挙動:

- 未完了・要再採点の受験者と、どの区分にも入らない受験者は、点数・区分が空欄になります。その場合は書き出す前に確認が表示されます。
- `fill` では、ファイルにいない受験者と、受験者一覧にいない行の数が、書き出したあとに表示されます。受験者一覧にいない行は変更しません。
- `read_as` に指定した名前が受験者インポート用フォーマットに見つからない、または `columns` の `header` が元のファイルの見出しに見つからないときはエラーになり、実際の見出し行の内容がエラーメッセージに表示されます。

### 付属フォーマット

| フォーマット | 方式 | 内容 |
| --- | --- | --- |
| 成績一覧 | 新規作成 | `id`, `name`, `score`, `rating` の 4 列（UTF-8、BOM 付き） |
| 成績一覧（Shift_JIS） | 新規作成 | 学籍番号・氏名・得点・評語の 4 列を Shift_JIS で |
| 合否一覧 | 新規作成 | 区分を合否（`合` / `否`）に置き換えて書き出す例 |
| Moodle 成績インポート | 新規作成 | Moodle の「成績 > インポート > CSV ファイル」で取り込む CSV |
| WebClass 外部データインポート | 新規作成 | WebClass の「外部データのインポート」用（`user_id`, `score`） |
| manaba 成績登録シート貼り付け用 | 新規作成 | manaba の成績登録シート（Excel）に貼り付けるための CSV |
| OneRoster results.csv | 新規作成 | OneRoster v1.1 の `results.csv`（課題IDと日付の列は空） |
| UTAS | 既存ファイルに記入 | UTAS の履修者名簿の `評語` 列に区分を書き込みます |
| UTOL 課題 | 既存ファイルに記入 | UTOL の課題提出状況 CSV の `評価` 列に最終得点を書き込みます |
| Moodle 評定ワークシート | 既存ファイルに記入 | 評定ワークシートの `Grade` 列に得点を書き込みます（そのままアップロードできます） |
| Canvas 成績表 | 既存ファイルに記入 | Canvas の成績エクスポート CSV の課題列に得点を書き込みます |
| Blackboard 成績センター | 既存ファイルに記入 | Blackboard の成績表ダウンロード CSV の成績列に得点を書き込みます |

- Canvas と Blackboard のフォーマットは、書き込む列の見出し（`header`）が課題ごとに違います。`<課題名>` `<列名>` を実際の見出し（例: `期末試験 (1234567)`）に書き換えてから使ってください。

## フォーマットを追加する（利用者向け）

アプリの「受験者設定」画面の **CSV フォーマット定義...**、成績評価画面の **定義...** ボタンを押すと、対応する定義ファイルが入っているフォルダが開きます。そのファイルを上記の仕様に沿って編集・追記してください。ファイルはフォーマットの選択欄を開くたびに読み直されるので、編集後にアプリを再起動する必要はありません。

JSON の文法エラーがある、または使えるフォーマットが 1 つも残っていないときは、画面に理由が表示され、このリポジトリに付属のフォーマットが使われます。

## フォーマットを追加する（開発者向け）

このフォルダの `import_formats.json` / `export_formats.json` に要素を追記してプルリクエストを送ってください。

- `cargo test` で、追記した要素が仕様どおりに読み書きできるかを確認します（`src/csv_import.rs` / `src/csv_export.rs` のテストが、ここに書かれているフォーマット全件を実際に読み書きして検証します）。壊れた JSON のままではビルド（テスト）が失敗します。
- 名前（`name`）は一覧内で重複しないようにしてください（重複するとテストが失敗します）。
- 対象システムの見出し名はバージョンや表示言語で変わることがあるので、確認した版と出典（マニュアルの URL など）をプルリクエストの説明に書いてください。
