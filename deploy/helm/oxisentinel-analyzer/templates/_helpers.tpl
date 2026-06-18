{{- define "oxisentinel-analyzer.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "oxisentinel-analyzer.fullname" -}}
{{- printf "%s-%s" .Release.Name (include "oxisentinel-analyzer.name" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}
