#!/bin/bash

# Prompt for commit message and branch
read -p "Commit message: " COMMIT_MSG
read -p "Branch to push to: " BRANCH

# Prompt for remote type
read -p "Use SSH remote? (y/n): " USE_SSH

if [[ "$USE_SSH" =~ ^[Yy]$ ]]; then
    REPO_URL="git@github.com:ND-Zyth/sysport.git"
else
    read -p "GitHub Username: " GITHUB_USER
    read -s -p "GitHub Token: " GITHUB_TOKEN
    echo
    # Trim whitespace
    GITHUB_USER=$(echo "$GITHUB_USER" | xargs)
    GITHUB_TOKEN=$(echo "$GITHUB_TOKEN" | xargs)
    if [[ -z "$GITHUB_USER" || -z "$GITHUB_TOKEN" ]]; then
        echo "Error: Username and token must not be empty."
        exit 1
    fi
    REPO_URL="https://$GITHUB_USER:$GITHUB_TOKEN@github.com/ND-Zyth/sysport.git"
fi

# Go to the root of the repo
cd "$(dirname "$0")/.."

# Add all files except this script
git add .
git reset scripts/upload.sh

git commit -m "$COMMIT_MSG"
git branch -M "$BRANCH"
git remote remove origin 2>/dev/null
git remote add origin https://github.com/ND-Zyth/sysport.git
git push -u origin "$BRANCH"

echo "Upload complete." 