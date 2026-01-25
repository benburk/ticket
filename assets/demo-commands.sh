tk init
echo "# Add user authentication" | tk create --type feat
echo "# Fix login bug" | tk create --type fix --priority 1
tk list
tk ready
