#!/system/bin/sh
set -eu

case "$0" in
    */service.sh) cd "${0%/*}" ;;
    *) exit 1 ;;
esac

case "${1:-}" in
    '') stage=service; attempt=.kver-service ;;
    --boot-completed) stage=boot_completed; attempt=.kver-boot ;;
    *) exit 1 ;;
esac
[ "$#" -le 1 ] || exit 1

for flag in disable remove; do
    if [ -e "$flag" ] || [ -L "$flag" ]; then
        exit 0
    fi
done

umask 077
# Reserve the only attempt before running any check.
mkdir -m 0700 "$attempt" 2>/dev/null || exit 0
(
    printf 'hook=entered\nstage=%s\n' "$stage"

    read_uptime() {
        uptime_value=
        if IFS=' ' read -r uptime_value _ 2>/dev/null < /proc/uptime &&
           [ "${#uptime_value}" -le 20 ]; then
            case "$uptime_value" in
                *.*)
                    whole=${uptime_value%%.*}; fraction=${uptime_value#*.}
                    case "$whole" in
                        ''|*[!0123456789]*) uptime_value=unavailable ;;
                    esac
                    case "$fraction" in
                        ''|*[!0123456789]*) uptime_value=unavailable ;;
                    esac
                    ;;
                *) uptime_value=unavailable ;;
            esac
        else
            uptime_value=unavailable
        fi
        printf '%s\n' "$uptime_value"
    }

    now_seconds() {
        current=$(read_uptime)
        case "$current" in
            unavailable) printf 'unavailable\n' ;;
            *) printf '%s\n' "${current%%.*}" ;;
        esac
    }

    emit_token() {
        label=$1; token_value=$2; token_set=$3; token_max=$4
        if [ "$token_set" != x ]; then
            printf '%s=unset\n' "$label"; return
        fi
        if [ -z "$token_value" ]; then
            printf '%s=empty\n' "$label"; return
        fi
        if [ "${#token_value}" -gt "$token_max" ]; then
            printf '%s=invalid\n' "$label"; return
        fi
        case "$token_value" in
            *[!A-Za-z0-9_.:/@+~-]*) printf '%s=invalid\n' "$label" ;;
            *) printf '%s=%s\n' "$label" "$token_value" ;;
        esac
    }

    emit_number() {
        label=$1; number_value=$2; number_set=$3
        if [ "$number_set" != x ]; then
            printf '%s=unset\n' "$label"; return
        fi
        if [ -z "$number_value" ]; then
            printf '%s=empty\n' "$label"; return
        fi
        case "$number_value" in
            *[!0123456789]*) printf '%s=invalid\n' "$label" ;;
            *)
                if [ "${#number_value}" -le 10 ]; then
                    printf '%s=%s\n' "$label" "$number_value"
                else
                    printf '%s=invalid\n' "$label"
                fi
                ;;
        esac
    }

    valid_field() {
        field_value=$1; field_kind=$2
        case "$field_kind" in
            number)
                case "$field_value" in
                    ''|*[!0123456789]*) return 1 ;;
                esac
                [ "${#field_value}" -le 10 ]
                ;;
            full)
                [ -n "$field_value" ] && [ "${#field_value}" -le 128 ] || return 1
                case "$field_value" in
                    *[!A-Za-z0-9_.+@~:/-]*) return 1 ;;
                esac
                ;;
            flags)
                case "$field_value" in
                    0x*|0X*) ;;
                    *) return 1 ;;
                esac
                hex=${field_value#0x}; hex=${hex#0X}
                [ -n "$hex" ] && [ "${#hex}" -le 8 ] || return 1
                case "$hex" in
                    *[!0123456789abcdefABCDEF]*) return 1 ;;
                esac
                ;;
            bool)
                case "$field_value" in
                    true|false) ;;
                    *) return 1 ;;
                esac
                ;;
            token)
                [ -n "$field_value" ] && [ "${#field_value}" -le 32 ] || return 1
                case "$field_value" in
                    *[!A-Za-z0-9_.-]*) return 1 ;;
                esac
                ;;
            *) return 1 ;;
        esac
    }

    uid=$(id -u 2>/dev/null) || uid=unavailable
    case "$uid" in
        ''|*[!0123456789]*) uid=unavailable ;;
    esac
    printf 'uid=%s\n' "$uid"

    emit_token env_ksu "${KSU-}" "${KSU+x}" 32
    emit_token env_bakasu "${BAKASU-}" "${BAKASU+x}" 32
    emit_token env_module "${KSU_MODULE-}" "${KSU_MODULE+x}" 64
    emit_number env_userspace_version "${KSU_VER_CODE-}" "${KSU_VER_CODE+x}"
    emit_number env_kernel_version "${KSU_KERNEL_VER_CODE-}" "${KSU_KERNEL_VER_CODE+x}"
    emit_number env_uapi_version "${KSU_UAPI_VER-}" "${KSU_UAPI_VER+x}"
    emit_token env_runtime_mode "${KSU_RUNTIME_MODE-}" "${KSU_RUNTIME_MODE+x}" 32

    context=
    if IFS= read -r context 2>/dev/null < "/proc/$$/attr/current" || [ -n "$context" ]; then
        case "$context" in
            ''|*[!a-zA-Z0-9_:.,-]*) printf 'selinux_context=invalid\n' ;;
            *)
                if [ "${#context}" -le 128 ]; then
                    printf 'selinux_context=%s\n' "$context"
                else
                    printf 'selinux_context=invalid\n'
                fi
                ;;
        esac
    else
        printf 'selinux_context=unavailable\n'
    fi
    for ns in mnt uts; do
        ns_target=$(timeout -s KILL 5 /system/bin/readlink "/proc/$$/ns/$ns" 2>/dev/null) || ns_target=
        case "$ns_target" in
            "$ns":\[*\])
                ns_id=${ns_target#*\[}; ns_id=${ns_id%\]}
                case "$ns_id" in
                    ''|*[!0123456789]*) ns_id=unavailable ;;
                    *) [ "${#ns_id}" -le 20 ] || ns_id=unavailable ;;
                esac
                ;;
            *) ns_id=unavailable ;;
        esac
        printf '%s_namespace=%s\n' "$ns" "$ns_id"
    done

    manager_path=/data/adb/ksud
    manager=verified
    if [ "$uid" != 0 ]; then
        manager=not_root
    elif [ ! -e "$manager_path" ]; then
        manager=absent
    elif [ -L "$manager_path" ]; then
        manager=symlink
    elif [ ! -f "$manager_path" ]; then
        manager=not_regular
    elif [ ! -x "$manager_path" ]; then
        manager=not_executable
    else
        manager_meta=$(stat -c '%u:%g:%a' "$manager_path" 2>/dev/null) || manager_meta=
        case "$manager_meta" in
            0:0:755) ;;
            '') manager=metadata_unavailable ;;
            *) manager=metadata_mismatch ;;
        esac
        if [ "$manager" = verified ]; then
            manager_hash=$(timeout -s KILL 5 /system/bin/sha256sum "$manager_path" 2>/dev/null) || manager_hash=
            case "${manager_hash%% *}" in
                fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50) ;;
                '') manager=hash_unavailable ;;
                *) manager=hash_mismatch ;;
            esac
        fi
    fi
    printf 'manager=%s\n' "$manager"

    environment=expected
    if [ "$uid" != 0 ] ||
       [ "${KSU:-}" != true ] ||
       [ "${BAKASU:-}" != true ] ||
       [ "${KSU_MODULE:-}" != earkey_control_kver ] ||
       [ "${KSU_VER_CODE:-}" != 35257 ] ||
       [ "${KSU_UAPI_VER:-}" != 6 ]; then
        environment=unexpected
    fi
    printf 'environment=%s\n' "$environment"

    emit_info_field() {
        info_label=$1; info_value=$2; info_count=$3; info_kind=$4
        if [ "$info_count" -eq 0 ]; then
            printf '%s=missing\n' "$info_label"; info_status=invalid; return
        fi
        if [ "$info_count" -ne 1 ]; then
            printf '%s=duplicate\n' "$info_label"; info_status=invalid; return
        fi
        if [ "$info_kind" = full ] && [ -z "$info_value" ]; then
            printf '%s=unknown\n' "$info_label"; return
        fi
        if valid_field "$info_value" "$info_kind"; then
            printf '%s=%s\n' "$info_label" "$info_value"
        else
            printf '%s=invalid\n' "$info_label"; info_status=invalid
        fi
    }

    parse_info_file() {
        info_file=$1
        info_version=; info_version_count=0
        info_full=; info_full_count=0
        info_flags=; info_flags_count=0
        info_uapi=; info_uapi_count=0
        info_lkm=; info_lkm_count=0
        info_bundled=; info_bundled_count=0
        info_late=; info_late_count=0
        info_mode=; info_mode_count=0
        info_pr=; info_pr_count=0
        while IFS= read -r info_line; do
            case "$info_line" in
                'version: '*) info_version=${info_line#version: }; info_version_count=$((info_version_count + 1)) ;;
                'full_version: '*) info_full=${info_line#full_version: }; info_full_count=$((info_full_count + 1)) ;;
                'flags: '*) info_flags=${info_line#flags: }; info_flags_count=$((info_flags_count + 1)) ;;
                'uapi_version: '*) info_uapi=${info_line#uapi_version: }; info_uapi_count=$((info_uapi_count + 1)) ;;
                'lkm: '*) info_lkm=${info_line#lkm: }; info_lkm_count=$((info_lkm_count + 1)) ;;
                'bundled: '*) info_bundled=${info_line#bundled: }; info_bundled_count=$((info_bundled_count + 1)) ;;
                'late_load: '*) info_late=${info_line#late_load: }; info_late_count=$((info_late_count + 1)) ;;
                'runtime_mode: '*) info_mode=${info_line#runtime_mode: }; info_mode_count=$((info_mode_count + 1)) ;;
                'pr_build: '*) info_pr=${info_line#pr_build: }; info_pr_count=$((info_pr_count + 1)) ;;
            esac
        done < "$info_file"

        info_status=complete
        emit_info_field info_version "$info_version" "$info_version_count" number
        emit_info_field info_full_version "$info_full" "$info_full_count" full
        emit_info_field info_flags "$info_flags" "$info_flags_count" flags
        emit_info_field info_uapi_version "$info_uapi" "$info_uapi_count" number
        emit_info_field info_lkm "$info_lkm" "$info_lkm_count" bool
        emit_info_field info_bundled "$info_bundled" "$info_bundled_count" bool
        emit_info_field info_late_load "$info_late" "$info_late_count" bool
        emit_info_field info_runtime_mode "$info_mode" "$info_mode_count" token
        emit_info_field info_pr_build "$info_pr" "$info_pr_count" bool
        printf 'info_status=%s\n' "$info_status"
    }

    parse_version_file() {
        version_file=$1
        kernel_version=; kernel_version_count=0; version_other=0
        while IFS= read -r version_line; do
            case "$version_line" in
                'Kernel Version: '*) kernel_version=${version_line#Kernel Version: }; kernel_version_count=$((kernel_version_count + 1)) ;;
                '') ;;
                *) version_other=1 ;;
            esac
        done < "$version_file"
        if [ "$kernel_version_count" -eq 0 ]; then
            printf 'version_status=missing\n'
        elif [ "$kernel_version_count" -ne 1 ] || [ "$version_other" -ne 0 ]; then
            printf 'version_status=invalid\n'
        elif valid_field "$kernel_version" number; then
            printf 'version_status=complete\nversion=%s\n' "$kernel_version"
        else
            printf 'version_status=invalid\n'
        fi
    }

    query_sample() {
        sample_number=$1
        printf 'sample=%s\n' "$sample_number"
        printf 'sample_uptime_seconds=%s\n' "$(read_uptime)"

        info_tmp=$attempt/.info.tmp
        info_result=failed
        if timeout -s KILL 3 "$manager_path" debug info > "$info_tmp" 2>/dev/null; then
            info_result=ok
        else
            info_rc=$?
            if [ "$info_rc" -eq 137 ]; then info_result=timeout; fi
        fi
        if [ "$info_result" = ok ]; then
            info_size=$(stat -c '%s' "$info_tmp" 2>/dev/null) || info_size=
            case "$info_size" in
                ''|*[!0123456789]*) info_result=invalid ;;
                *) if [ "$info_size" -gt 4096 ]; then info_result=oversize; fi ;;
            esac
        fi
        printf 'info_result=%s\n' "$info_result"
        if [ "$info_result" = ok ]; then
            parse_info_file "$info_tmp"
        fi
        rm -f "$info_tmp"

        version_tmp=$attempt/.version.tmp
        version_result=failed
        if timeout -s KILL 3 "$manager_path" debug version > "$version_tmp" 2>/dev/null; then
            version_result=ok
        else
            version_rc=$?
            if [ "$version_rc" -eq 137 ]; then version_result=timeout; fi
        fi
        if [ "$version_result" = ok ]; then
            version_size=$(stat -c '%s' "$version_tmp" 2>/dev/null) || version_size=
            case "$version_size" in
                ''|*[!0123456789]*) version_result=invalid ;;
                *) if [ "$version_size" -gt 512 ]; then version_result=oversize; fi ;;
            esac
        fi
        printf 'version_result=%s\n' "$version_result"
        if [ "$version_result" = ok ]; then
            parse_version_file "$version_tmp"
        fi
        rm -f "$version_tmp"
    }

    if [ "$environment" = expected ] && [ "$manager" = verified ]; then
        printf 'query=sampled\n'
        case "$stage" in
            service) max_samples=8 ;;
            boot_completed) max_samples=1 ;;
            *) max_samples=0 ;;
        esac
        sample_number=0
        sampling=complete
        sample_start=$(now_seconds)
        if [ "$sample_start" = unavailable ]; then sampling=clock_unavailable; fi
        while [ "$sample_number" -lt "$max_samples" ]; do
            if [ "$sample_start" != unavailable ]; then
                sample_now=$(now_seconds)
                if [ "$sample_now" != unavailable ] &&
                   [ "$sample_now" -ge $((sample_start + 10)) ]; then
                    sampling=deadline
                    break
                fi
            fi
            sample_number=$((sample_number + 1))
            query_sample "$sample_number"
            if [ "$sample_number" -lt "$max_samples" ]; then
                sleep 1
            fi
        done
        printf 'sample_count=%s\n' "$sample_number"
        printf 'sampling=%s\n' "$sampling"
    elif [ "$environment" != expected ]; then
        printf 'query=skipped_environment\n'
    else
        printf 'query=skipped_manager\n'
    fi

    dmesg=unavailable
    dmesg_tmp=$attempt/.dmesg.tmp
    dmesg_status=$attempt/.dmesg.status
    if command -v dmesg >/dev/null 2>&1; then
        dmesg_pipeline=ok
        (
            dmesg_rc=0
            timeout -s KILL 5 dmesg 2>/dev/null || dmesg_rc=$?
            printf '%s\n' "$dmesg_rc" > "$dmesg_status"
        ) | { grep -E 'Initialized with driver version|ksu_change_ksuver|KernelSU|kernelsu|ksu' || true; } | tail -n 400 | cut -c1-512 > "$dmesg_tmp" || dmesg_pipeline=failed
        dmesg_rc=
        if [ -f "$dmesg_status" ]; then IFS= read -r dmesg_rc < "$dmesg_status" || dmesg_rc=; fi
        if [ "$dmesg_pipeline" = ok ] && [ "$dmesg_rc" = 0 ]; then
            dmesg=available
        fi
    fi
    printf 'dmesg=%s\n' "$dmesg"
    if [ "$dmesg" = available ]; then
        dmesg_init_count=0
        dmesg_init_state=absent
        dmesg_init_version=; dmesg_init_full=; dmesg_init_abi=
        dmesg_init_mode=; dmesg_init_uapi=
        dmesg_change_count=0
        dmesg_change_values=
        dmesg_change_invalid=0
        dmesg_change_overflow=0
        while IFS= read -r dmesg_line; do
            case "$dmesg_line" in
                *"Initialized with driver version: "*", full_version: "*", ABI: "*", Work mode: "*", ksu_uapi_version: "*)
                    init_rest=${dmesg_line#*Initialized with driver version: }
                    init_version=${init_rest%%, full_version: *}
                    init_rest=${init_rest#*, full_version: }
                    init_full=${init_rest%%, ABI: *}
                    init_rest=${dmesg_line#*, ABI: }
                    init_abi=${init_rest%%, Work mode: *}
                    init_rest=${dmesg_line#*, Work mode: }
                    init_mode=${init_rest%%, ksu_uapi_version: *}
                    init_rest=${dmesg_line#*, ksu_uapi_version: }
                    init_uapi=${init_rest%%[!0123456789]*}
                    dmesg_init_count=$((dmesg_init_count + 1))
                    if valid_field "$init_version" number &&
                       valid_field "$init_full" full &&
                       valid_field "$init_abi" token &&
                       valid_field "$init_mode" token &&
                       valid_field "$init_uapi" number; then
                        dmesg_init_state=present
                        dmesg_init_version=$init_version
                        dmesg_init_full=$init_full
                        dmesg_init_abi=$init_abi
                        dmesg_init_mode=$init_mode
                        dmesg_init_uapi=$init_uapi
                    else
                        dmesg_init_state=invalid
                    fi
                    ;;
                *"ksu_change_ksuver to: "*)
                    change_value=${dmesg_line##*ksu_change_ksuver to: }
                    case "$change_value" in
                        ''|*[!0123456789]*) dmesg_change_invalid=1 ;;
                        *)
                            if [ "${#change_value}" -le 10 ]; then
                                dmesg_change_count=$((dmesg_change_count + 1))
                                if [ "$dmesg_change_count" -le 16 ]; then
                                    if [ -n "$dmesg_change_values" ]; then
                                        dmesg_change_values="$dmesg_change_values,$change_value"
                                    else
                                        dmesg_change_values=$change_value
                                    fi
                                else
                                    dmesg_change_overflow=1
                                fi
                            else
                                dmesg_change_invalid=1
                            fi
                            ;;
                    esac
                    ;;
            esac
        done < "$dmesg_tmp"
        printf 'dmesg_init_count=%s\n' "$dmesg_init_count"
        printf 'dmesg_init=%s\n' "$dmesg_init_state"
        if [ "$dmesg_init_state" = present ]; then
            printf 'dmesg_init_version=%s\n' "$dmesg_init_version"
            printf 'dmesg_init_full_version=%s\n' "$dmesg_init_full"
            printf 'dmesg_init_abi=%s\n' "$dmesg_init_abi"
            printf 'dmesg_init_work_mode=%s\n' "$dmesg_init_mode"
            printf 'dmesg_init_uapi_version=%s\n' "$dmesg_init_uapi"
        fi
        printf 'dmesg_change_ksuver_count=%s\n' "$dmesg_change_count"
        if [ -n "$dmesg_change_values" ]; then
            printf 'dmesg_change_ksuver_values=%s\n' "$dmesg_change_values"
        else
            printf 'dmesg_change_ksuver_values=none\n'
        fi
        printf 'dmesg_change_ksuver_invalid=%s\n' "$dmesg_change_invalid"
        printf 'dmesg_change_ksuver_overflow=%s\n' "$dmesg_change_overflow"
    fi
    rm -f "$dmesg_tmp" "$dmesg_status"

    proc_modules=unavailable
    if [ -r /proc/modules ]; then
        proc_modules=available
        proc_module_count=0
        proc_module_seen=0
        proc_module_list=
        proc_module_invalid=0
        while IFS=' ' read -r module_name module_size _ 2>/dev/null; do
            proc_module_seen=$((proc_module_seen + 1))
            if [ "$proc_module_seen" -gt 256 ]; then
                proc_modules=truncated
                break
            fi
            case "$module_name" in
                kernelsu|ksu|resukisu|susfs)
                    if valid_field "$module_size" number; then
                        proc_module_count=$((proc_module_count + 1))
                        if [ -n "$proc_module_list" ]; then
                            proc_module_list="$proc_module_list,$module_name:$module_size"
                        else
                            proc_module_list="$module_name:$module_size"
                        fi
                    else
                        proc_module_invalid=1
                    fi
                    ;;
            esac
        done < /proc/modules
        printf 'proc_modules=%s\n' "$proc_modules"
        printf 'proc_module_count=%s\n' "$proc_module_count"
        if [ -n "$proc_module_list" ]; then
            printf 'proc_module_list=%s\n' "$proc_module_list"
        else
            printf 'proc_module_list=none\n'
        fi
        printf 'proc_module_invalid=%s\n' "$proc_module_invalid"
    else
        printf 'proc_modules=unavailable\n'
    fi

    config=unavailable
    config_tmp=$attempt/.config.tmp
    config_path=/proc/config.gz
    busybox_bin=/data/adb/ksu/bin/busybox
    if [ -r "$config_path" ] && [ -x "$busybox_bin" ]; then
        if timeout -s KILL 5 "$busybox_bin" zcat "$config_path" > "$config_tmp" 2>/dev/null; then
            config_size=$(stat -c '%s' "$config_tmp" 2>/dev/null) || config_size=
            case "$config_size" in
                ''|*[!0123456789]*) config=invalid ;;
                *)
                    if [ "$config_size" -gt 1048576 ]; then
                        config=oversize
                    elif [ "$config_size" -eq 0 ]; then
                        config=empty
                    else
                        config=available
                    fi
                    ;;
            esac
        fi
    fi
    printf 'config=%s\n' "$config"
    if [ "$config" = available ]; then
        config_ksu_count=0; config_ksu_value=
        config_toolkit_count=0; config_toolkit_value=
        config_susfs_count=0; config_susfs_value=
        config_multi_count=0; config_multi_value=
        while IFS= read -r config_line; do
            case "$config_line" in
                CONFIG_KSU=*) config_ksu_value=${config_line#CONFIG_KSU=}; config_ksu_count=$((config_ksu_count + 1)) ;;
                '# CONFIG_KSU is not set') config_ksu_value=n; config_ksu_count=$((config_ksu_count + 1)) ;;
                CONFIG_KSU_TOOLKIT_SUPPORT=*) config_toolkit_value=${config_line#CONFIG_KSU_TOOLKIT_SUPPORT=}; config_toolkit_count=$((config_toolkit_count + 1)) ;;
                '# CONFIG_KSU_TOOLKIT_SUPPORT is not set') config_toolkit_value=n; config_toolkit_count=$((config_toolkit_count + 1)) ;;
                CONFIG_KSU_SUSFS=*) config_susfs_value=${config_line#CONFIG_KSU_SUSFS=}; config_susfs_count=$((config_susfs_count + 1)) ;;
                '# CONFIG_KSU_SUSFS is not set') config_susfs_value=n; config_susfs_count=$((config_susfs_count + 1)) ;;
                CONFIG_KSU_MULTI_MANAGER_SUPPORT=*) config_multi_value=${config_line#CONFIG_KSU_MULTI_MANAGER_SUPPORT=}; config_multi_count=$((config_multi_count + 1)) ;;
                '# CONFIG_KSU_MULTI_MANAGER_SUPPORT is not set') config_multi_value=n; config_multi_count=$((config_multi_count + 1)) ;;
            esac
        done < "$config_tmp"
        emit_config() {
            config_label=$1; config_count=$2; config_value=$3
            if [ "$config_count" -eq 0 ]; then
                printf 'config_%s=absent\n' "$config_label"; return
            fi
            if [ "$config_count" -ne 1 ]; then
                printf 'config_%s=duplicate\n' "$config_label"; return
            fi
            case "$config_value" in
                y|m|n) printf 'config_%s=%s\n' "$config_label" "$config_value" ;;
                *) printf 'config_%s=invalid\n' "$config_label" ;;
            esac
        }
        emit_config ksu "$config_ksu_count" "$config_ksu_value"
        emit_config ksu_toolkit_support "$config_toolkit_count" "$config_toolkit_value"
        emit_config ksu_susfs "$config_susfs_count" "$config_susfs_value"
        emit_config ksu_multi_manager_support "$config_multi_count" "$config_multi_value"
    else
        printf 'config_ksu=unavailable\n'
        printf 'config_ksu_toolkit_support=unavailable\n'
        printf 'config_ksu_susfs=unavailable\n'
        printf 'config_ksu_multi_manager_support=unavailable\n'
    fi
    rm -f "$config_tmp"

    manager_version_file=absent
    manager_version_path=/data/adb/ksu/.manager_version
    if [ -L "$manager_version_path" ]; then
        manager_version_file=symlink
    elif [ -e "$manager_version_path" ]; then
        if [ ! -f "$manager_version_path" ]; then
            manager_version_file=not_regular
        else
            manager_version_file=present
        fi
    fi
    printf 'manager_version_file=%s\n' "$manager_version_file"
    if [ "$manager_version_file" = present ]; then
        mv_owner=$(stat -c '%u:%g' "$manager_version_path" 2>/dev/null) || mv_owner=
        case "$mv_owner" in
            '') printf 'manager_version_owner=unavailable\n' ;;
            0:0) printf 'manager_version_owner=0:0\n' ;;
            *) printf 'manager_version_owner=mismatch\n' ;;
        esac
        mv_mode=$(stat -c '%a' "$manager_version_path" 2>/dev/null) || mv_mode=
        case "$mv_mode" in
            ''|*[!01234567]*) printf 'manager_version_mode=unavailable\n' ;;
            *)
                if [ "${#mv_mode}" -ge 3 ] && [ "${#mv_mode}" -le 4 ]; then
                    printf 'manager_version_mode=%s\n' "$mv_mode"
                else
                    printf 'manager_version_mode=invalid\n'
                fi
                ;;
        esac
        mv_size=$(stat -c '%s' "$manager_version_path" 2>/dev/null) || mv_size=
        case "$mv_size" in
            ''|*[!0123456789]*) printf 'manager_version_size=unavailable\n' ;;
            *) printf 'manager_version_size=%s\n' "$mv_size" ;;
        esac
        mv_value=
        if IFS= read -r mv_value 2>/dev/null < "$manager_version_path" || [ -n "$mv_value" ]; then
            if valid_field "$mv_value" number; then
                printf 'manager_version_value=%s\n' "$mv_value"
            else
                printf 'manager_version_value=invalid\n'
            fi
        else
            printf 'manager_version_value=unavailable\n'
        fi
    fi

    modules=absent
    if [ -d /data/adb/modules ]; then
        modules=available
        module_count=0
        module_names=
        module_names_truncated=0
        module_names_invalid=0
        for module_dir in /data/adb/modules/*; do
            if [ ! -d "$module_dir" ]; then
                continue
            fi
            module_count=$((module_count + 1))
            if [ "$module_count" -gt 64 ]; then
                module_names_truncated=1
                break
            fi
            module_name=${module_dir##*/}
            case "$module_name" in
                ''|*[!A-Za-z0-9._-]*) module_names_invalid=1; continue ;;
            esac
            if [ "${#module_name}" -gt 64 ]; then
                module_names_invalid=1; continue
            fi
            if [ -n "$module_names" ]; then
                module_names="$module_names,$module_name"
            else
                module_names=$module_name
            fi
        done
        printf 'modules=%s\n' "$modules"
        printf 'module_count=%s\n' "$module_count"
        if [ -n "$module_names" ]; then
            printf 'module_names=%s\n' "$module_names"
        else
            printf 'module_names=none\n'
        fi
        printf 'module_names_truncated=%s\n' "$module_names_truncated"
        printf 'module_names_invalid=%s\n' "$module_names_invalid"
    else
        printf 'modules=absent\n'
        printf 'module_count=0\n'
        printf 'module_names=none\n'
        printf 'module_names_truncated=0\n'
        printf 'module_names_invalid=0\n'
    fi

    presence_state() {
        presence_path=$1
        if [ -L "$presence_path" ]; then
            printf 'symlink'
        elif [ ! -e "$presence_path" ]; then
            printf 'absent'
        elif [ -f "$presence_path" ]; then
            printf 'present'
        else
            printf 'not_regular'
        fi
    }

    toolkit_dir=/data/adb/modules/ksu_toolkit
    if [ -L "$toolkit_dir" ]; then
        toolkit_dir_state=symlink
    elif [ ! -e "$toolkit_dir" ]; then
        toolkit_dir_state=absent
    elif [ -d "$toolkit_dir" ]; then
        toolkit_dir_state=present
    else
        toolkit_dir_state=not_directory
    fi
    printf 'ksu_toolkit_dir=%s\n' "$toolkit_dir_state"
    printf 'ksu_toolkit_toolkit=%s\n' "$(presence_state /data/adb/modules/ksu_toolkit/toolkit)"
    printf 'ksu_toolkit_system_bin=%s\n' "$(presence_state /data/adb/modules/ksu_toolkit/system/bin/toolkit)"

    printf 'record=complete\n'
) > "$attempt/record.tmp" 2>&1 || true
mv "$attempt/record.tmp" "$attempt/record"
