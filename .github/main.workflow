workflow "On Push" {
  resolves = ["Google Cloud Build", "Build Fuzzers"]
  on = "push"
}

action "Setup Google Cloud" {
  uses = "actions/gcloud/auth@master"
  secrets = ["GCLOUD_AUTH"]
}

action "Build and Test Image" {
  uses = "actions/docker/cli@master"
  args = ["build -t zebrad ."]
  secrets = ["GCLOUD_AUTH"]
}

action "Google Cloud Build" {
  needs = ["Setup Google Cloud"]
  uses = "actions/gcloud/cli@master"
  runs = "sh -l -c"
  args = "SHORT_SHA=$(echo ${GITHUB_SHA} | head -c7) && BRANCH_NAME=$(git name-rev --name-only HEAD) && gcloud builds submit . --config cloudbuild.yaml --project zebrad --substitutions BRANCH_NAME=$BRANCH_NAME,SHORT_SHA=$SHORT_SHA"
}

# Filter for master branch
action "if branch = master:" {
  needs = ["Build and Test Image"]
  uses = "actions/bin/filter@master"
  args = "branch master"
}

action "Tag image for GCR" {
  needs = ["if branch = master:"]
  uses = "actions/docker/tag@master"
  args = ["zebrad", "gcr.io/zebrad/master"]
}

action "Set Credential Helper for Docker" {
  needs = ["if branch = master:", "Setup Google Cloud"]
  uses = "actions/gcloud/cli@master"
  args = ["auth", "configure-docker", "--quiet"]
}

action "Push image to GCR" {
  needs = ["Tag image for GCR", "Set Credential Helper for Docker"]
  uses = "actions/gcloud/cli@master"
  runs = "sh -c"
  args = ["docker push gcr.io/zebrad/master"]
}

action "Build Fuzzers" {
  needs = ["Push image to GCR"]
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
