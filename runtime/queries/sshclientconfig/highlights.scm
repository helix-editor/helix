(host) @keyword
(host_value) @identifier

(match) @keyword
(match_value) @identifier

(add_keys_to_agent) @keyword
(add_keys_to_agent_value) @boolean

(address_family) @keyword
(address_family_value) @type

(batch_mode) @keyword
(batch_mode_value) @boolean

(bind_address) @keyword
(bind_address_value) @string

(bind_interface) @keyword
(bind_interface_value) @string

(canonical_domains) @keyword
(canonical_domains_value) @identifier

(canonicalize_fallback_local) @keyword
(canonicalize_fallback_local_value) @boolean

(canonicalize_hostname) @keyword
(canonicalize_hostname_value) @boolean

(canonicalize_max_dots) @keyword
(canonicalize_max_dots_value) @number

(canonicalize_permitted_cnames) @keyword
(canonicalize_permitted_cnames_value) @identifier

(ca_signature_algorithms) @keyword
(ca_signature_algorithms_value) @identifier

(certificate_file) @keyword
(certificate_file_value) @file

(challenge_response_authentication) @keyword
(challenge_response_authentication_value) @boolean

(check_host_ip) @keyword
(check_host_ip_value) @boolean

(cipher) @keyword
(cipher_value) @identifier

(ciphers) @keyword
(ciphers_value) @identifier

(clear_all_forwardings) @keyword
(clear_all_forwardings_value) @boolean

(comment) @comment

(compression) @keyword
(compression_value) @boolean

(connect_timeout) @keyword
(connect_timeout_value) @number

(connection_attempts) @keyword
(connection_attempts_value) @number

(control_master) @keyword
(control_master_value) @type

(control_path) @keyword
(control_path_value) @file

(control_persist) @keyword
(control_persist_value) @type

(dynamic_forward) @keyword
(dynamic_forward_value) @string

(enable_ssh_keysign) @keyword
(enable_ssh_keysign_value) @boolean

(escape_char) @keyword
(escape_char_value) @string

(exit_on_forward_failure) @keyword
(exit_on_forward_failure_value) @boolean

(fingerprint_hash) @keyword
(fingerprint_hash_value) @identifier

(fork_after_authentication) @keyword
(fork_after_authentication_value) @boolean

(forward_agent) @keyword
(forward_agent_value) @boolean

(forward_x11) @keyword
(forward_x11_value) @boolean

(forward_x11_timeout) @keyword
(forward_x11_timeout_value) @time

(forward_x11_trusted) @keyword
(forward_x11_trusted_value) @boolean

(gateway_ports) @keyword
(gateway_ports_value) @boolean

(global_known_hosts_file) @keyword
(global_known_hosts_file_value) @file

(gssapi_authentication) @keyword
(gssapi_authentication_value) @boolean

(gssapi_client_identity) @keyword
(gssapi_client_identity_value) @string

(gssapi_delegate_credentials) @keyword
(gssapi_delegate_credentials_value) @boolean

(gssapi_kex_algorithms) @keyword
(gssapi_kex_algorithms_value) @identifier

(gssapi_key_exchange) @keyword
(gssapi_key_exchange_value) @boolean

(gssapi_renewal_forces_rekey) @keyword
(gssapi_renewal_forces_rekey_value) @boolean

(gssapi_server_identity) @keyword
(gssapi_server_identity_value) @string

(gssapi_trust_dns) @keyword
(gssapi_trust_dns_value) @boolean

(hash_known_hosts) @keyword
(hash_known_hosts_value) @boolean

(host_key_algorithms) @keyword
(host_key_algorithms_value) @identifier

(host_key_alias) @keyword
(host_key_alias_value) @string

(hostbased_accepted_algorithms) @keyword
(hostbased_accepted_algorithms_value) @identifier

(hostbased_authentication) @keyword
(hostbased_authentication_value) @boolean

(hostname) @keyword
(hostname_value) @string

(identities_only) @keyword
(identities_only_value) @boolean

(identity_agent) @keyword
(identity_agent_value) @string

(identity_file) @keyword
(identity_file_value) @file

(ignore_unknown) @keyword
(ignore_unknown_value) @string

(include) @keyword
(include_value) @file

(ip_qos) @keyword
(ip_qos_value) @type

(kbd_interactive_authentication) @keyword
(kbd_interactive_authentication_value) @boolean

(kbd_interactive_devices) @keyword
(kbd_interactive_devices_value) @type

(kex_algorithms) @keyword
(kex_algorithms_value) @identifier

(known_hosts_command) @keyword
(known_hosts_command_value) @string

(local_command) @keyword
(local_command_value) @string

(local_forward) @keyword
(local_forward_value) @string

(log_level) @keyword
(log_level_value) @type

(log_verbose) @keyword
(log_verbose_value) @string

(macs) @keyword
(macs_value) @identifier

(no_host_authentication_for_localhost) @keyword
(no_host_authentication_for_localhost_value) @boolean

(number_of_password_prompts) @keyword
(number_of_password_prompts_value) @number

(password_authentication) @keyword
(password_authentication_value) @boolean

(permit_local_command) @keyword
(permit_local_command_value) @boolean

(permit_remote_open) @keyword
(permit_remote_open_value) @string

(pkcs11_provider) @keyword
(pkcs11_provider_value) @string

(port) @keyword
(port_value) @number

(preferred_authentications) @keyword
(preferred_authentications_value) @type

(protocol) @keyword
(protocol_value) @number

(proxy_command) @keyword
(proxy_command_value) @string

(proxy_jump) @keyword
(proxy_jump_value) @string

(proxy_use_fdpass) @keyword
(proxy_use_fdpass_value) @boolean

(pubkey_accepted_algorithms) @keyword
(pubkey_accepted_algorithms_value) @identifier

(pubkey_accepted_key_types) @keyword
(pubkey_accepted_key_types_value) @identifier

(pubkey_authentication) @keyword
(pubkey_authentication_value) @boolean

(rekey_limit) @keyword
(rekey_limit_value) @string

(remote_command) @keyword
(remote_command_value) @string

(remote_forward) @keyword
(remote_forward_value) @string

(request_tty) @keyword
(request_tty_value) @type

(revoked_host_keys) @keyword
(revoked_host_keys_value) @file

(security_key_provider) @keyword
(security_key_provider_value) @string

(send_env) @keyword
(send_env_value) @string

(server_alive_count_max) @keyword
(server_alive_count_max_value) @number

(server_alive_interval) @keyword
(server_alive_interval_value) @number

(session_type) @keyword
(session_type_value) @type

(set_env) @keyword
(set_env_value) @string

(stdin_null) @keyword
(stdin_null_value) @boolean

(stream_local_bind_mask) @keyword
(stream_local_bind_mask_value) @string

(stream_local_bind_unlink) @keyword
(stream_local_bind_unlink_value) @boolean

(strict_host_key_checking) @keyword
(strict_host_key_checking_value) @type

(syslog_facility) @keyword
(syslog_facility_value) @type

(tcp_keep_alive) @keyword
(tcp_keep_alive_value) @boolean
(keep_alive) @keyword
(keep_alive_value) @boolean

(tunnel) @keyword
(tunnel_value) @type

(tunnel_device) @keyword
(tunnel_device_value) @string

(update_host_keys) @keyword
(update_host_keys_value) @type

(use_keychain) @keyword
(use_keychain_value) @boolean

(user) @keyword
(user_value) @string

(user_known_hosts_file) @keyword
(user_known_hosts_file_value) @file

(verify_host_key_dns) @keyword
(verify_host_key_dns_value) @type

(visual_host_key) @keyword
(visual_host_key_value) @boolean

(xauth_location) @keyword
(xauth_location_value) @file
