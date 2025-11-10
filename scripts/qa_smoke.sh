#!/usr/bin/env bash
# QA post-flash smoke sequence covering provisioning, Wi-Fi credentials, OTA upload, and auth guard checks.
set -euo pipefail

BASE_URL="http://192.168.4.1"
POST_WIFI_URL=""
OTA_IMAGE=""
LOGIN_USER=""
LOGIN_PASS=""
PROVISION_USER=""
PROVISION_PASS=""
WIFI_SSID=""
WIFI_PASSWORD=""
RUN_NEGATIVE=1
POLL_TIMEOUT=180
POLL_INTERVAL=3

usage() {
    cat <<'EOF'
Usage: qa_smoke.sh --ota-image <path> --login-user <user> --login-pass <pass> [options]

Required:
  --ota-image PATH        Firmware image to upload (<=2 MiB).
  --login-user USER       Admin username for login.
  --login-pass PASS       Admin password for login.

Optional:
  --base-url URL          Device URL before any Wi-Fi change (default http://192.168.4.1).
  --post-wifi-url URL     Expected URL after Wi-Fi creds apply (set if STA IP differs).
  --provision-user USER   Username to set during first-run provisioning (default login user).
  --provision-pass PASS   Password to set during provisioning (default login pass).
  --wifi-ssid SSID        Wi-Fi network to push to the device (skips Wi-Fi step if empty).
  --wifi-pass PASS        Wi-Fi password (omit or blank for open network).
  --skip-negative         Skip 401/400 negative OTA regression checks.
  --poll-timeout SEC      Override wait timeout for reboot/network recovery (default 180).
  --poll-interval SEC     Override polling interval (default 3).
  -h, --help              Show this help.

Examples:
  BASE_URL=http://192.168.4.1 scripts/qa_smoke.sh \
    --ota-image build/erik.bin \
    --login-user admin --login-pass firmware123 \
    --wifi-ssid TestNet --wifi-pass supersecret \
    --post-wifi-url http://192.168.1.120
EOF
}

log() {
    printf '[%(%Y-%m-%dT%H:%M:%SZ)T] %s\n' -1 "$1"
}

require_cmd() {
    for cmd in "$@"; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            echo "Missing required command: $cmd" >&2
            exit 1
        fi
    done
}

curl_json() {
    local method="$1"
    local url="$2"
    local data="${3:-}"
    local auth_header="${4:-}"
    local content_type="${5:-application/json}"
    local tmp_body tmp_headers
    tmp_body=$(mktemp)
    tmp_headers=$(mktemp)
    local args=("-sS" "--show-error" "-X" "$method" "$url" "-D" "$tmp_headers" "-o" "$tmp_body" "-w" '%{http_code}')
    if [[ -n "$auth_header" ]]; then
        args+=("-H" "$auth_header")
    fi
    if [[ -n "$data" ]]; then
        args+=("-H" "Content-Type: $content_type" "--data" "$data")
    fi
    HTTP_STATUS=$(curl "${args[@]}")
    BODY=$(cat "$tmp_body")
    rm -f "$tmp_body" "$tmp_headers"
}

curl_binary() {
    local method="$1"
    local url="$2"
    local file="$3"
    local auth_header="${4:-}"
    local tmp_body tmp_headers
    tmp_body=$(mktemp)
    tmp_headers=$(mktemp)
    local args=("-sS" "--show-error" "-X" "$method" "$url" "-D" "$tmp_headers" "-o" "$tmp_body" "-w" '%{http_code}' "-H" "Content-Type: application/octet-stream" "--data-binary" "@$file")
    if [[ -n "$auth_header" ]]; then
        args+=("-H" "$auth_header")
    fi
    HTTP_STATUS=$(curl "${args[@]}")
    BODY=$(cat "$tmp_body")
    rm -f "$tmp_body" "$tmp_headers"
}

check_status() {
    local context="$1"
    if [[ ${HTTP_STATUS:0:1} != "2" ]]; then
        echo "${context} failed (HTTP ${HTTP_STATUS}): ${BODY}" >&2
        exit 1
    fi
    log "$context succeeded (HTTP ${HTTP_STATUS})"
}

wait_for_http() {
    local url="$1"
    local timeout="${2:-$POLL_TIMEOUT}"
    local interval="${3:-$POLL_INTERVAL}"
    local elapsed=0
    log "Waiting for $url"
    while (( elapsed < timeout )); do
        if curl -sS --max-time 2 "$url" >/dev/null 2>&1; then
            log "Endpoint reachable"
            return 0
        fi
        sleep "$interval"
        elapsed=$((elapsed + interval))
    done
    echo "Timed out waiting for $url" >&2
    return 1
}

obtain_token() {
    local payload
    payload=$(jq -n --arg user "$LOGIN_USER" --arg pass "$LOGIN_PASS" '{username:$user, password:$pass}')
    curl_json POST "$CURRENT_BASE_URL/api/login" "$payload"
    check_status "Login"
    TOKEN=$(echo "$BODY" | jq -r '.token // empty')
    if [[ -z "$TOKEN" ]]; then
        echo "Login response missing token: $BODY" >&2
        exit 1
    fi
    log "Obtained bearer token"
}

maybe_provision() {
    curl_json GET "$CURRENT_BASE_URL/api/provisioning"
    check_status "Provisioning check"
    local provisioned
    provisioned=$(echo "$BODY" | jq -r '.provisioned // false')
    if [[ "$provisioned" == "true" ]]; then
        log "Device already provisioned"
        return
    fi
    log "Device not provisioned; provisioning with user '$PROVISION_USER'"
    local payload
    payload=$(jq -n --arg user "$PROVISION_USER" --arg pass "$PROVISION_PASS" '{username:$user, password:$pass}')
    curl_json POST "$CURRENT_BASE_URL/api/provisioning" "$payload"
    check_status "Provisioning"
    TOKEN=$(echo "$BODY" | jq -r '.token // empty')
    if [[ -z "$TOKEN" ]]; then
        echo "Provisioning response missing token: $BODY" >&2
        exit 1
    fi
    log "Provisioning completed and session token issued"
}

post_wifi_relogin() {
    if [[ -n "$POST_WIFI_URL" ]]; then
        CURRENT_BASE_URL="$POST_WIFI_URL"
        log "Switching to post-Wi-Fi base URL: $CURRENT_BASE_URL"
    fi
    wait_for_http "$CURRENT_BASE_URL/" "$POLL_TIMEOUT" "$POLL_INTERVAL"
    obtain_token
}

negative_expect() {
    local expected="$1"
    local description="$2"
    shift 2
    local tmp_body tmp_headers
    tmp_body=$(mktemp)
    tmp_headers=$(mktemp)
    local status
    status=$(curl -sS -X POST "$CURRENT_BASE_URL/api/admin/firmware" "$@" -D "$tmp_headers" -o "$tmp_body" -w '%{http_code}' || true)
    local body
    body=$(cat "$tmp_body")
    rm -f "$tmp_body" "$tmp_headers"
    if [[ "$status" != "$expected" ]]; then
        echo "$description expected HTTP $expected but saw $status: $body" >&2
        exit 1
    fi
    log "$description produced HTTP $status as expected"
}

ARGS=("$@")
while [[ $# -gt 0 ]]; do
    case "$1" in
        --ota-image)
            OTA_IMAGE="$2"; shift 2;;
        --login-user)
            LOGIN_USER="$2"; shift 2;;
        --login-pass)
            LOGIN_PASS="$2"; shift 2;;
        --base-url)
            BASE_URL="$2"; shift 2;;
        --post-wifi-url)
            POST_WIFI_URL="$2"; shift 2;;
        --provision-user)
            PROVISION_USER="$2"; shift 2;;
        --provision-pass)
            PROVISION_PASS="$2"; shift 2;;
        --wifi-ssid)
            WIFI_SSID="$2"; shift 2;;
        --wifi-pass)
            WIFI_PASSWORD="$2"; shift 2;;
        --skip-negative)
            RUN_NEGATIVE=0; shift;;
        --poll-timeout)
            POLL_TIMEOUT="$2"; shift 2;;
        --poll-interval)
            POLL_INTERVAL="$2"; shift 2;;
        -h|--help)
            usage; exit 0;;
        *)
            echo "Unknown option: $1" >&2
            usage
            exit 1;;
    esac
    set -- "$@"
done

if [[ -z "$OTA_IMAGE" || -z "$LOGIN_USER" || -z "$LOGIN_PASS" ]]; then
    echo "--ota-image, --login-user, and --login-pass are required." >&2
    usage
    exit 1
fi

if [[ ! -f "$OTA_IMAGE" ]]; then
    echo "OTA image not found: $OTA_IMAGE" >&2
    exit 1
fi

if [[ -z "$PROVISION_USER" ]]; then
    PROVISION_USER="$LOGIN_USER"
fi
if [[ -z "$PROVISION_PASS" ]]; then
    PROVISION_PASS="$LOGIN_PASS"
fi

require_cmd curl jq sha256sum stat mktemp head dd

CURRENT_BASE_URL="$BASE_URL"
TOKEN=""

wait_for_http "$CURRENT_BASE_URL/" "$POLL_TIMEOUT" "$POLL_INTERVAL"
maybe_provision
if [[ -z "$TOKEN" ]]; then
    obtain_token
fi

curl_json GET "$CURRENT_BASE_URL/api/status" "" "Authorization: Bearer $TOKEN"
check_status "Baseline status"
echo "$BODY" | jq '.'

if [[ -n "$WIFI_SSID" ]]; then
    log "Pushing Wi-Fi credentials for SSID '$WIFI_SSID'"
    local_payload=$(jq -n --arg ssid "$WIFI_SSID" --arg pass "$WIFI_PASSWORD" 'if ($pass | length) > 0 then {ssid:$ssid, password:$pass} else {ssid:$ssid} end')
    curl_json POST "$CURRENT_BASE_URL/api/wifi" "$local_payload" "Authorization: Bearer $TOKEN"
    check_status "Wi-Fi save"
    log "Wi-Fi save response: $BODY"
    sleep 5
    post_wifi_relogin
    curl_json GET "$CURRENT_BASE_URL/api/status" "" "Authorization: Bearer $TOKEN"
    check_status "Status after Wi-Fi reboot"
    echo "$BODY" | jq '.wifi'
fi

IMAGE_SIZE=$(stat --format=%s "$OTA_IMAGE")
IMAGE_SHA=$(sha256sum "$OTA_IMAGE" | awk '{print $1}')
log "Uploading OTA image ($IMAGE_SIZE bytes, sha256=$IMAGE_SHA)"
curl_binary POST "$CURRENT_BASE_URL/api/admin/firmware" "$OTA_IMAGE" "Authorization: Bearer $TOKEN"
check_status "OTA upload"
log "OTA response: $BODY"

sleep 5
wait_for_http "$CURRENT_BASE_URL/" "$POLL_TIMEOUT" "$POLL_INTERVAL"
obtain_token
curl_json GET "$CURRENT_BASE_URL/api/status" "" "Authorization: Bearer $TOKEN"
check_status "Post-OTA status"
STATUS_SHA=$(echo "$BODY" | jq -r '.firmware.sha256 // empty')
STATUS_SIZE=$(echo "$BODY" | jq -r '.firmware.size // 0')
if [[ -z "$STATUS_SHA" ]]; then
    echo "Firmware metadata missing after OTA: $BODY" >&2
    exit 1
fi
if [[ "$STATUS_SHA" != "$IMAGE_SHA" ]]; then
    echo "Firmware sha mismatch: expected $IMAGE_SHA got $STATUS_SHA" >&2
    exit 1
fi
if [[ "$STATUS_SIZE" != "$IMAGE_SIZE" ]]; then
    echo "Firmware size mismatch: expected $IMAGE_SIZE got $STATUS_SIZE" >&2
    exit 1
fi
log "Firmware metadata matches uploaded image"

echo "$BODY" | jq '.firmware'

if (( RUN_NEGATIVE )); then
    log "Running negative OTA checks"
    negative_expect 401 "OTA without bearer token" -H "Content-Type: application/octet-stream" --data-binary "@$OTA_IMAGE"
    negative_expect 400 "OTA wrong content-type" -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" --data-binary "@$OTA_IMAGE"
    truncated=$(mktemp)
    head -c 512 "$OTA_IMAGE" > "$truncated"
    negative_expect 400 "OTA truncated image" -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/octet-stream" --data-binary "@$truncated"
    bogus=$(mktemp)
    dd if=/dev/zero of="$bogus" bs=1 count=512 >/dev/null 2>&1
    negative_expect 400 "OTA missing ESP magic byte" -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/octet-stream" --data-binary "@$bogus"
    rm -f "$truncated" "$bogus"
    curl_json GET "$CURRENT_BASE_URL/api/status" "" "Authorization: Bearer $TOKEN"
    check_status "Status after negative cases"
    log "Firmware metadata after negatives: $(echo "$BODY" | jq -r '.firmware.sha256')"
fi

log "QA smoke suite complete"
