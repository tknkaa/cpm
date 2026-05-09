package main

import (
	"encoding/json"
	"fmt"
	"math"
	"os/exec"
	"strings"
	"time"

	"charm.land/lipgloss/v2"
	"github.com/charmbracelet/x/term"
)

type QuotaSnapshot struct {
	QuotaRemaining float64 `json:"quota_remaining"`
	Unlimited      bool    `json:"unlimited"`
	Entitlement    int     `json:"entitlement"`
}

type QuotaSnapshots struct {
	Chat                QuotaSnapshot `json:"chat"`
	Completions         QuotaSnapshot `json:"completions"`
	PremiumInteractions QuotaSnapshot `json:"premium_interactions"`
}

type Response struct {
	QuotaResetDate string         `json:"quota_reset_date"`
	QuotaSnapshots QuotaSnapshots `json:"quota_snapshots"`
}

func renderBar(percent float64, width int, color string) string {
	filled := int(math.Round(float64(width) * percent / 100))
	filledStyle := lipgloss.NewStyle().Background(lipgloss.Color(color))
	emptyStyle := lipgloss.NewStyle().Background(lipgloss.Color("240"))
	return filledStyle.Render(strings.Repeat(" ", filled)) + emptyStyle.Render(strings.Repeat(" ", width-filled))
}

func render(result Response) {
	fmt.Println()
	termWidth, _, _ := term.GetSize(0)

	cyan := lipgloss.NewStyle().Foreground(lipgloss.Color("14"))
	plain := lipgloss.NewStyle()

	resetDate := result.QuotaResetDate

	t, _ := time.Parse("2006-01-02", resetDate)
	daysRemaining := int(time.Until(t).Hours()/24) + 1
	now := time.Now()
	daysInMonth := time.Date(now.Year(), now.Month()+1, 0, 0, 0, 0, 0, time.UTC).Day()
	elapsed := now.Day()
	elapsedPercent := float64(elapsed) / float64(daysInMonth) * 100

	header := cyan.Render(" GitHub Copilot Quota")
	fmt.Println(header)
	fmt.Println(strings.Repeat("\u2500", termWidth))

	p := result.QuotaSnapshots.PremiumInteractions
	used := p.Entitlement - int(p.QuotaRemaining)
	usedPercent := float64(used) / float64(p.Entitlement) * 100
	var status string
	barColor := "#00D787"
	diff := usedPercent - elapsedPercent

	if diff > 15 {
		status = "Overusing"
		barColor = "#FF0000"
	} else if diff > 5 {
		status = "Slightly fast"
	} else if diff < -10 {
		status = "Plenty left"
	} else {
		status = "Good pace"
	}

	barWidth := termWidth - 2
	percentRemaining := p.QuotaRemaining / float64(p.Entitlement) * 100
	fmt.Printf(" Premium Requests %d/%d used (%.1f%% remaining)\n",
		used, p.Entitlement, percentRemaining)
	fmt.Println(" " + renderBar(100-percentRemaining, barWidth, barColor))
	fmt.Println(strings.Repeat("\u2500", termWidth))

	fmt.Printf(" Month Progress %d/%d days elapsed (%.1f%% elapsed)\n", elapsed, daysInMonth, elapsedPercent)
	fmt.Println(" " + renderBar(elapsedPercent, barWidth, "#0EA5E9"))
	fmt.Println(strings.Repeat("\u2500", termWidth))

	perDay := p.QuotaRemaining / float64(daysRemaining)
	fmt.Println(plain.Render(fmt.Sprintf(" Reset: %s (%d days remaining)", resetDate, daysRemaining)))
	fmt.Println(" Status: " + status)
	fmt.Println(cyan.Render(fmt.Sprintf(" You can use up to %.1f premium requests per day until reset", perDay)))
}

func main() {
	cmd := exec.Command("gh", "api", "/copilot_internal/user")
	out, err := cmd.Output()
	if err != nil {
		fmt.Println("Error executing command:", err)
		return
	}
	var result Response
	if err := json.Unmarshal(out, &result); err != nil {
		fmt.Println("Error parsing JSON:", err)
		return
	}
	render(result)
}
