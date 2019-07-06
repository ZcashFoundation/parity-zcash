workflow "On Push" {
  resolves = ["Build Fuzzers"]
  on = "push"
}

action "Setup Google Cloud" {
  uses = "actions/gcloud/auth@master"
  secrets = ["GCLOUD_AUTH"]
}

action "Build, Test, Tag, and Push to GCR" {
  needs = ["Setup Google Cloud"]
  uses = "actions/gcloud/cli@master"
  runs = "sh -l -c"
  args = ["SHORT_SHA=$(echo $GITHUB_SHA | head -c7) && BRANCH_NAME=$(git name-rev --name-only HEAD) && gcloud builds submit . --config cloudbuild.yaml --project zebrad --substitutions BRANCH_NAME=$BRANCH_NAME"]
}

# Filter for master branch
action "if branch = master:" {
  needs = ["Build, Test, Tag, and Push to GCR"]
  uses = "actions/bin/filter@master"
  args = "branch master"
}

action "Build Fuzzers" {
  needs = ["if branch = master:"]
  uses = "docker://gcr.io/zebrad/master:latest"
  runs = ["sh", "-c", "cd fuzz/; cargo install --force afl honggfuzz; cargo hfuzz build; cargo afl build; exit 0"]
}

# action "Benchmark" {
# needs = ["if branch = master:"]
#   uses = "docker://gcr.io/zebrad/master:latest"
#   runs = "./tools/bench.sh"
# }


# action "Doc" {
#   needs = ["if branch = master:"]
#   uses = "docker://gcr.io/zebrad/master:latest"
#   runs = "cargo doc"
# }
