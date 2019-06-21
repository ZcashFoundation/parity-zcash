workflow "On Push" {
  on = "push"
  resolves = ["Push image to GCR"]
}

action "Build and Test Image" {
  uses = "actions/docker/cli@master"
  args = ["build -t zebrad ."]
  secrets = ["GCLOUD_AUTH"]
}

# Filter for master branch
action "if branch = master:" {
  needs = ["Build and Test Image"]
  uses = "actions/bin/filter@master"
  args = "branch master"
}

action "Setup Google Cloud" {
  needs = ["if branch = master:"]
  uses = "actions/gcloud/auth@master"
  secrets = ["GCLOUD_AUTH"]
}

action "Tag image for GCR" {
  needs = ["if branch = master:"]
  uses = "actions/docker/tag@master"
  args = ["zebrad", "gcr.io/zebrad/master"]
}

action "Set Credential Helper for Docker" {
  needs = ["Setup Google Cloud"]
  uses = "actions/gcloud/cli@master"
  args = ["auth", "configure-docker", "--quiet"]
}

action "Push image to GCR" {
  needs = ["Tag image for GCR", "Set Credential Helper for Docker"]
  uses = "actions/gcloud/cli@master"
  runs = "sh -c"
  args = ["docker push gcr.io/zebrad/master"]
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

action "Build Fuzzers" {
  needs = ["Push image to GCR"]
  uses = "docker://gcr.io/zebrad/master:latest"
  runs = ["sh", "-c", "cd fuzz/ && cargo install --force afl honggfuzz && cargo hfuzz build && cargo afl build"]
}
