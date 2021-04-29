use anyhow::{Context, Result};
use rusoto_ec2::{
    AuthorizeSecurityGroupIngressRequest, Ec2, Ec2Client, IpPermission, IpRange,
    RevokeSecurityGroupIngressRequest,
};

pub struct Handler<'a> {
    client: &'a Ec2Client,
}

impl<'a> Handler<'a> {
    pub fn new(client: &'a Ec2Client) -> Self {
        Self { client }
    }

    pub async fn grant_access(
        &self,
        security_group_id: String,
        description: Option<String>,
    ) -> Result<()> {
        let mut request = AuthorizeSecurityGroupIngressRequest::default();
        request.group_id = Some(security_group_id);
        request.ip_permissions = Some(vec![self.get_ip_permission(description).await?]);

        self.client
            .authorize_security_group_ingress(request)
            .await
            .context("Failed to add public ip to allowlist")?;

        Ok(())
    }

    pub async fn revoke_access(&self, security_group_id: String) -> Result<()> {
        let mut request = RevokeSecurityGroupIngressRequest::default();
        request.group_id = Some(security_group_id);
        request.ip_permissions = Some(vec![self.get_ip_permission(None).await?]);

        self.client
            .revoke_security_group_ingress(request)
            .await
            .context("Failed to remove public ip from allowlist")?;

        Ok(())
    }

    async fn get_ip_permission(&self, description: Option<String>) -> Result<IpPermission> {
        let ip = public_ip::addr()
            .await
            .context("Unable to determine public ip")?;

        Ok(IpPermission {
            from_port: Some(22),
            to_port: Some(22),
            ip_protocol: Some("tcp".into()),
            ip_ranges: Some(vec![IpRange {
                cidr_ip: Some(format!("{}/32", ip)),
                description,
            }]),
            ipv_6_ranges: None,
            prefix_list_ids: None,
            user_id_group_pairs: None,
        })
    }
}
