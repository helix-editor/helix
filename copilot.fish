# Downloading Copilot lsp
function clone_copilot_lsp
    echo "*** Cloning the copilot lsp into /usr/local/bin ***"
    set -l url "https://github.com/github/copilot.vim"
    set -l folder_name "dist/"
    set -l temp_dir (mktemp -d)

    if test -d "/usr/local/bin/$folder_name"
        echo "Folder '$folder_name' already exists in /usr/local/bin. Delete this to re-install. Aborting."
        return 1
    end

    echo "Cloning repository from $url.."
    if not git clone $url $temp_dir
        echo "Failed to clone repository."
    else if not sudo mv $temp_dir/$folder_name /usr/local/bin/$folder_name
        echo "Failed to move $folder_name to /usr/local/bin."
    end
    echo "Moving $folder_name to /usr/local/bin.."

    echo "Cleaning up..."
    rm -rf $temp_dir
end

function create_copilot_exe
    set -l target_binary "/usr/local/bin/copilot"
    echo "*** Creating an executable in /usr/local/bin to call the copilot lsp ***"
    if test -x $target_binary
        echo "The file '$target_binary' already exists. Delete this to re-install. Aborting."
        return
    end
    echo '#! /usr/bin/env bash' > $target_binary
    echo 'node /usr/local/bin/dist/language-server.js' >>$target_binary
    sudo chmod +x $target_binary
    echo "The executable 'copilot' has been created at $target_binary"
end

# Auth
function get_value 
    set -l pairs (string split '&' $argv[1])
    set -l kv_line (string match -r "$argv[2]=.*" $pairs)
    echo (string split "=" $kv_line)[2]
end

function get_device_params
    set -l payload '{ "scope": "read:user", "client_id": "Iv1.b507a08c87ecfe98" }'
    set -l resp (curl -s -X POST "https://github.com/login/device/code" \
        -H "Content-Type: application/json" \
        -d $payload)
    echo $resp
end

function oath 
    set -l payload "{\"client_id\": \"Iv1.b507a08c87ecfe98\", \"device_code\": \"$argv[1]\", \"grant_type\": \"urn:ietf:params:oauth:grant-type:device_code\"}"
    set -l resp (curl -s -X POST "https://github.com/login/oauth/access_token" \
                    -H "Content-Type: application/json" \
                    -d $payload)
    echo $resp
end

function auth
    echo "*** Creating a github access-key file for copilot in ~/.config/github-copilot/hosts.json ***"
    set file_path ~/.config/github-copilot/hosts.json
    if test -e $file_path
        echo "Copilot access-key file $file_path already exists. Delete this to re-install/re-authenticate .Aborting."
        return 1
    end
    
    set device_params (get_device_params)
    set device_code (get_value $device_params device_code)
    set uri (get_value $device_params uri)
    set uri (echo $uri | sed 's/%/\\\\x/g')
    set user_code (get_value $device_params user_code)

    echo -e "Go to $uri and enter the code $user_code"

    while true 
        set resp (oath $device_code)

        set access_token (get_value $resp access_token)
        set error (get_value $resp error)
        if not test -z $access_token
            mkdir -p ~/.config/github-copilot
            echo "{\"github.com\":{\"oauth_token\":\"$access_token\"}}" > ~/.config/github-copilot/hosts.json
            echo "Copilot access_token written to ~/.config/github-copilot/hosts.json"
            return 0
        else if not test -z error
            echo $error
        else 
            echo "Bad response."
            return 1
        end
        sleep 5
    end
end


# Main 
if not set -q argv[1]
    echo "No argument provided. Use --create-lsp or --auth."
    exit 1
end

switch $argv[1]
    case --create-lsp
        clone_copilot_lsp
        echo ""
        create_copilot_exe
    case --auth
        auth
    case '*'
        echo "Unknown option: $argv[1]"
        exit 1
end
