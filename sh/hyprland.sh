function grab_active_window() {
    local active_window=`hyprctl -j activewindow`
    local box=$(echo $active_window | jq -r '"\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"' | cut -f1,2 -d' ')

    echo "$box"
}

grab_active_window
