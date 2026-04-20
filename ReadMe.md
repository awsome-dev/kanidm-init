### 初期化コマンド

podman exec kanidm /sbin/kanidm_init --config-path /data/server.toml --setup-config-path /data/setup.toml

------

podman exec kanidmd /sbin/kanidmd recover-account idm_admin --config-path /data/server.toml
kanidm login --name idm_admin --url https://idm.example.internal --accept-invalid-certs
kanidm system oauth2 create internal_admin_portal "Internal Admin Portal" "https://admin.idm.example.internal/ui/admin/login" -H https://idm.example.internal --accept-invalid-certs
kanidm system oauth2 add-redirect-url internal_admin_portal https://admin.idm.example.internal/callback -H https://idm.example.internal --accept-invalid-certs
kanidm system oauth2 update-scope-map internal_admin_portal idm_admins email profile openid -H https://idm.example.internal --accept-invalid-certs
kanidm system oauth2 get internal_admin_portal -H https://idm.example.internal --accept-invalid-certs
