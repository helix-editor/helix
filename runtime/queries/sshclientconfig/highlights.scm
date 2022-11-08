(host) @namespace
(host_value) @string

(match) @namespace
(match_value) @string

(add_keys_to_agent) @keyword
(add_keys_to_agent_value) @constant.builtin.boolean

(address_family) @keyword
(address_family_value) @constant.builtin

(batch_mode) @keyword
(batch_mode_value) @constant.builtin.boolean

(bind_address) @keyword
(bind_address_value) @string

(bind_interface) @keyword
(bind_interface_value) @string

(canonical_domains) @keyword
(canonical_domains_value) @string

(canonicalize_fallback_local) @keyword
(canonicalize_fallback_local_value) @constant.builtin.boolean

(canonicalize_hostname) @keyword
(canonicalize_hostname_value) @constant.builtin

(canonicalize_max_dots) @keyword
(canonicalize_max_dots_value) @constant.numeric.integer

(canonicalize_permitted_cnames) @keyword
(canonicalize_permitted_cnames_value) @string

(ca_signature_algorithms) @keyword
(ca_signature_algorithms_value) @string

(certificate_file) @keyword
(certificate_file_value) @string.special.path

(challenge_response_authentication) @keyword
(challenge_response_authentication_value) @constant.builtin.boolean

(check_host_ip) @keyword
(check_host_ip_value) @constant.builtin.boolean

(cipher) @keyword
(cipher_value) @string

(ciphers) @keyword
(ciphers_value) @string

(clear_all_forwardings) @keyword
(clear_all_forwardings_value) @constant.builtin.boolean

(comment) @comment

(compression) @keyword
(compression_value) @constant.builtin.boolean

(connect_timeout) @keyword
(connect_timeout_value) @constant.numeric.integer

(connection_attempts) @keyword
(connection_attempts_value) @constant.numeric.integer

(control_master) @keyword
(control_master_value) @constant.builtin

(control_path) @keyword
(control_path_value) @string.special.path

(control_persist) @keyword
(control_persist_value) @constant.builtin

(dynamic_forward) @keyword
(dynamic_forward_value) @string

(enable_ssh_keysign) @keyword
(enable_ssh_keysign_value) @constant.builtin.boolean

(escape_char) @keyword
(escape_char_value) @constant.character.escape

(exit_on_forward_failure) @keyword
(exit_on_forward_failure_value) @constant.builtin.boolean

(fingerprint_hash) @keyword
(fingerprint_hash_value) @constant.builtin

(fork_after_authentication) @keyword
(fork_after_authentication_value) @constant.builtin.boolean

(forward_agent) @keyword
(forward_agent_value) @string

(forward_x11) @keyword
(forward_x11_value) @constant.builtin.boolean

(forward_x11_timeout) @keyword
(forward_x11_timeout_value) @constant.numeric.integer

(forward_x11_trusted) @keyword
(forward_x11_trusted_value) @constant.builtin.boolean

(gateway_ports) @keyword
(gateway_ports_value) @constant.builtin.boolean

(global_known_hosts_file) @keyword
(global_known_hosts_file_value) @string.special.path

(gssapi_authentication) @keyword
(gssapi_authentication_value) @constant.builtin.boolean

(gssapi_client_identity) @keyword
(gssapi_client_identity_value) @string

(gssapi_delegate_credentials) @keyword
(gssapi_delegate_credentials_value) @constant.builtin.boolean

(gssapi_kex_algorithms) @keyword
(gssapi_kex_algorithms_value) @string

(gssapi_key_exchange) @keyword
(gssapi_key_exchange_value) @constant.builtin.boolean

(gssapi_renewal_forces_rekey) @keyword
(gssapi_renewal_forces_rekey_value) @constant.builtin.boolean

(gssapi_server_identity) @keyword
(gssapi_server_identity_value) @string

(gssapi_trust_dns) @keyword
(gssapi_trust_dns_value) @constant.builtin.boolean

(hash_known_hosts) @keyword
(hash_known_hosts_value) @constant.builtin.boolean

(host_key_algorithms) @keyword
(host_key_algorithms_value) @string

(host_key_alias) @keyword
(host_key_alias_value) @string

(hostbased_accepted_algorithms) @keyword
(hostbased_accepted_algorithms_value) @string

(hostbased_authentication) @keyword
(hostbased_authentication_value) @constant.builtin.boolean

(hostname) @keyword
(hostname_value) @string

(identities_only) @keyword
(identities_only_value) @constant.builtin.boolean

(identity_agent) @keyword
(identity_agent_value) @string

(identity_file) @keyword
(identity_file_value) @string.special.path

(ignore_unknown) @keyword
(ignore_unknown_value) @string

(include) @function.macro
(include_value) @string.special.path

(ip_qos) @keyword
(ip_qos_value) @constant.builtin

(kbd_interactive_authentication) @keyword
(kbd_interactive_authentication_value) @constant.builtin.boolean

(kbd_interactive_devices) @keyword
(kbd_interactive_devices_value) @string

(kex_algorithms) @keyword
(kex_algorithms_value) @string

(known_hosts_command) @keyword
(known_hosts_command_value) @string

(local_command) @keyword
(local_command_value) @string

(local_forward) @keyword
(local_forward_value) @string

(log_level) @keyword
(log_level_value) @constant.builtin

(log_verbose) @keyword
(log_verbose_value) @string

(macs) @keyword
(macs_value) @string

(no_host_authentication_for_localhost) @keyword
(no_host_authentication_for_localhost_value) @constant.builtin.boolean

(number_of_password_prompts) @keyword
(number_of_password_prompts_value) @constant.numeric.integer

(password_authentication) @keyword
(password_authentication_value) @constant.builtin.boolean

(permit_local_command) @keyword
(permit_local_command_value) @constant.builtin.boolean

(permit_remote_open) @keyword
(permit_remote_open_value) @string

(pkcs11_provider) @keyword
(pkcs11_provider_value) @string

(port) @keyword
(port_value) @constant.numeric.integer

(preferred_authentications) @keyword
(preferred_authentications_value) @string

(protocol) @keyword
(protocol_value) @constant.numeric.integer

(proxy_command) @keyword
(proxy_command_value) @string

(proxy_jump) @keyword
(proxy_jump_value) @string

(proxy_use_fdpass) @keyword
(proxy_use_fdpass_value) @constant.builtin.boolean

(pubkey_accepted_algorithms) @keyword
(pubkey_accepted_algorithms_value) @string

(pubkey_accepted_key_types) @keyword
(pubkey_accepted_key_types_value) @string

(pubkey_authentication) @keyword
(pubkey_authentication_value) @constant.builtin

(rekey_limit) @keyword
(rekey_limit_value) @string

(remote_command) @keyword
(remote_command_value) @string

(remote_forward) @keyword
(remote_forward_value) @string

(request_tty) @keyword
(request_tty_value) @constant.builtin

(revoked_host_keys) @keyword
(revoked_host_keys_value) @string.special.path

(security_key_provider) @keyword
(security_key_provider_value) @string

(send_env) @keyword
(send_env_value) @string

(server_alive_count_max) @keyword
(server_alive_count_max_value) @constant.numeric.integer

(server_alive_interval) @keyword
(server_alive_interval_value) @constant.numeric.integer

(session_type) @keyword
(session_type_value) @constant.builtin

(set_env) @keyword
(set_env_value) @string

(stdin_null) @keyword
(stdin_null_value) @constant.builtin.boolean

(stream_local_bind_mask) @keyword
(stream_local_bind_mask_value) @string

(stream_local_bind_unlink) @keyword
(stream_local_bind_unlink_value) @constant.builtin.boolean

(strict_host_key_checking) @keyword
(strict_host_key_checking_value) @constant.builtin

(syslog_facility) @keyword
(syslog_facility_value) @constant.builtin

(tcp_keep_alive) @keyword
(tcp_keep_alive_value) @constant.builtin.boolean
(keep_alive) @keyword
(keep_alive_value) @constant.builtin.boolean

(tunnel) @keyword
(tunnel_value) @constant.builtin

(tunnel_device) @keyword
(tunnel_device_value) @string

(update_host_keys) @keyword
(update_host_keys_value) @constant.builtin

(use_keychain) @keyword
(use_keychain_value) @constant.builtin.boolean

(user) @keyword
(user_value) @string

(user_known_hosts_file) @keyword
(user_known_hosts_file_value) @string.special.path

(verify_host_key_dns) @keyword
(verify_host_key_dns_value) @constant.builtin

(visual_host_key) @keyword
(visual_host_key_value) @constant.builtin.boolean

(xauth_location) @keyword
(xauth_location_value) @string.special.path
