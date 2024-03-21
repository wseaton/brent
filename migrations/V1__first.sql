{% with vault = make_vault_client() %}
{% for item in vault.list(
        'apps',
        'my-team'
    ) %}
SELECT
    '{{ item }}' AS env,
    '{{ get_env("VAULT_ROLE_ID") }}' AS role_id,
    {% if not loop.last %}
    UNION ALL
    {% endif %}
{% endfor %};
SELECT
    '{{ vault.get('apps', 'my-team/snowflake/scim')['SCIM_TOKEN'] }}' AS a_secret;
{% endwith %}
