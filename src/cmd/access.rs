use anyhow::{Context, Result};
use rusoto_core::region;
use rusoto_ec2::{
    AuthorizeSecurityGroupIngressRequest, Ec2, Ec2Client, IpPermission, IpRange,
    RevokeSecurityGroupIngressRequest,
};

pub async fn grant(security_group_id: String, description: Option<String>) -> Result<()> {
    let client = Ec2Client::new(region::Region::UsEast1);
    let mut request = AuthorizeSecurityGroupIngressRequest::default();
    request.group_id = Some(security_group_id);
    request.ip_permissions = Some(vec![get_ip_permission(description).await?]);

    client
        .authorize_security_group_ingress(request)
        .await
        .context("Failed to add public ip to allowlist")?;

    Ok(())
}

pub async fn revoke(security_group_id: String) -> Result<()> {
    let client = Ec2Client::new(region::Region::UsEast1);
    let mut request = RevokeSecurityGroupIngressRequest::default();
    request.group_id = Some(security_group_id);
    request.ip_permissions = Some(vec![get_ip_permission(None).await?]);

    client
        .revoke_security_group_ingress(request)
        .await
        .context("Failed to remove public ip from allowlist")?;

    Ok(())
}

async fn get_ip_permission(description: Option<String>) -> Result<IpPermission> {
    let ip = public_ip::addr()
        .await
        .context("Unable to determine public ip")?;

    Ok(IpPermission {
        from_port: Some(22),
        to_port: Some(22),
        ip_protocol: Some("tcp".into()),
        ip_ranges: Some(vec![IpRange {
            cidr_ip: Some(format!("{}/32", ip)),
            description: description,
        }]),
        ipv_6_ranges: None,
        prefix_list_ids: None,
        user_id_group_pairs: None,
    })
}
