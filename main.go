package main

import (
	"fmt"
	"strings"
	"time"
)

const (
	// カーソル制御
	CursorHide  = "\x1b[?25l"
	CursorShow  = "\x1b[?25h"
	ReturnStart = "\r"

	// 色設定（ここが変更点！）
	BgGreen = "\x1b[42m" // 背景を緑にする
	BgReset = "\x1b[0m"  // 色設定（背景・文字）をリセット
)

func main() {
	total := 50 // バーの長さ（スペースの個数）

	fmt.Print(CursorHide)
	defer fmt.Print(CursorShow)

	fmt.Println("背景色を使ったプログレスバー:")

	for i := 0; i <= 100; i++ {
		percent := i
		filledLen := (total * percent) / 100

		// 1. 進捗部分は「背景緑」＋「スペース」
		// 背景色が緑になっているので、ただの空白が「緑のブロック」に見える
		filled := BgGreen + strings.Repeat(" ", filledLen)

		// 2. 未完了部分は「リセット（デフォルト背景）」＋「スペース」
		// ここは何も色がついていないただの空白になる
		empty := BgReset + strings.Repeat(" ", total-filledLen)

		// 3. 描画
		// \r で戻る -> 緑の空白 -> 普通の空白 -> 数字
		fmt.Printf("%s%s%s %d%%", ReturnStart, filled, empty, percent)

		time.Sleep(50 * time.Millisecond)
	}

	fmt.Println("\nDone!")
}
