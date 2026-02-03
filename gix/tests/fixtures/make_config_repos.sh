#!/usr/bin/env bash
set -eu -o pipefail

git init http-config
(cd http-config
  git config http.extraHeader "ExtraHeader: value1"
  git config --add http.extraHeader ""
  git config --add http.extraHeader "ExtraHeader: value2"
  git config --add http.extraHeader "ExtraHeader: value3"
  git config http.followRedirects true
  git config http.lowSpeedLimit 5k
  git config http.lowSpeedTime 10
  git config http.postBuffer 8k
  git config http.proxy http://localhost:9090
  git config http.proxyAuthMethod basic
  git config http.userAgent agentJustForHttp
  git config gitoxide.http.connectTimeout 60k
  git config http.schannelCheckRevoke true
  git config http.sslCAInfo ./CA.pem
  git config http.sslVersion sslv2
  git config http.version HTTP/1.1
)

git clone --shared http-config http-remote-override
(cd http-remote-override

  git config http.followRedirects initial

  git config http.proxy http://localhost:9090
  git config http.proxyAuthMethod basic

  git config remote.origin.proxy overridden
  git config remote.origin.proxyAuthMethod negotiate
)

git init http-no-proxy
(cd http-no-proxy
  git config gitoxide.http.noProxy "no validation done here"
)

git init http-ssl-version-min-max
(cd http-ssl-version-min-max
  git config http.sslVersion sslv3
  git config gitoxide.http.sslVersionMin tlsv1.1
  git config gitoxide.http.sslVersionMax tlsv1.2
)

git init http-ssl-version-default
(cd http-ssl-version-default
  git config http.sslVersion default
)

git init http-disabled-cainfo
(cd http-disabled-cainfo
  git config http.sslCAInfo ./CA.pem
  git config http.schannelUseSSLCAInfo false
)

git init http-proxy-empty
(cd http-proxy-empty
  git config http.followRedirects false

  git config http.proxy localhost:9090
  git config --add http.proxy "" # a value override disabling it later
)

git init ssh-all-options
(cd ssh-all-options
  git config ssh.variant ssh
  git config core.sshCommand "ssh -VVV"
  git config gitoxide.ssh.commandWithoutShellFallback "does not matter as it is a fallback"
)

git init ssh-command-fallback
(cd ssh-command-fallback
  git config ssh.variant putty
  git config gitoxide.ssh.commandWithoutShellFallback "ssh --fallback"
)

git init https-proxy-only
(cd https-proxy-only
  git config gitoxide.https.proxy https
)

git init gitoxide-http-proxy-only
(cd gitoxide-http-proxy-only
  git config gitoxide.http.proxy http-fallback
)

git init gitoxide-all-proxy-only
(cd gitoxide-all-proxy-only
  git config gitoxide.http.allProxy all-proxy-fallback
)

git init gitoxide-all-proxy
(cd gitoxide-all-proxy
  git config http.proxy http
  git config gitoxide.http.allProxy all-proxy-fallback
)

git init gitoxide-http-proxy
(cd gitoxide-http-proxy
  git config gitoxide.http.proxy http-fallback
  git config http.proxy http
)

git init https-proxy
(cd https-proxy
  git config gitoxide.https.proxy https
  git config --add http.proxy "http"  # only for HTTP urls
)

git init https-proxy-empty
(cd https-proxy-empty
  git config gitoxide.https.proxy https
  git config --add gitoxide.https.proxy "" # empty strings disable it
)

git init http-proxy-auto-prefix
(cd http-proxy-auto-prefix
  git config http.proxy localhost:9090 # http:// is prefixed automatically
)

git init http-verbose
(cd http-verbose
  git config gitoxide.http.verbose true
)

git init http-proxy-authenticated
(cd http-proxy-authenticated
  git config http.proxy user@localhost:9090
  cat <<EOF >> .git/config
[http]
  followRedirects
EOF
)

git init object-caches
(cd object-caches
  git config core.deltaBaseCacheLimit 128m
  git config gitoxide.objects.cacheLimit 16m
)

git init disabled-object-caches
(cd disabled-object-caches
  git config core.deltaBaseCacheLimit 0
  git config gitoxide.objects.cacheLimit 0
)

git init --bare bare-no-config
(cd bare-no-config
  rm config
  git config -l >/dev/null
)

git init worktree-no-config
(cd worktree-no-config
  rm .git/config
  git config -l >/dev/null
)

mkdir not-a-repo-empty-dir;
mkdir not-a-repo-with-files;
(cd not-a-repo-with-files
  touch this that
)

git init ssl-verify-disabled
(cd ssl-verify-disabled
  git config http.sslVerify false
)

git init ssl-no-verify-enabled
(cd ssl-no-verify-enabled
  git config http.sslVerify true
  git config gitoxide.http.sslNoVerify true
)


git init --bare with-hasconfig
(cd with-hasconfig
  cat >include-with-url <<-\EOF
  [user]
    name = "works"
    email = "works@example.com"
EOF
  cat >system.config <<-\EOF
  [includeIf "hasconfig:remote.*.url:anyurl"]
    path = "include-with-url"
EOF

  echo $'[remote "any"]\n\turl=anyurl' >>config

)

git init big-file-threshold
(cd big-file-threshold
  git config core.bigFileThreshold 42
)

git init --bare bare-repo
mkdir bare-link
(cd bare-link
  echo "gitdir: ../bare-repo" >.git
  echo hi >file-ignored-by-status
)
