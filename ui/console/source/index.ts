export type ConsoleStatus = 'ready' | 'degraded'

export interface ConsoleSummary {
  serviceName: string
  status: ConsoleStatus
  detail?: string
}

export function formatConsoleSummary(summary: ConsoleSummary): string {
  const detail = summary.detail?.trim()
  const fields = [summary.serviceName, summary.status]

  if (detail) {
    fields.push(detail)
  }

  return fields.join(' | ')
}
