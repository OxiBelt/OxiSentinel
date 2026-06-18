{{- define "oxisentinel-gateway-controller.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "oxisentinel-gateway-controller.fullname" -}}
{{- printf "%s-%s" .Release.Name (include "oxisentinel-gateway-controller.name" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}
