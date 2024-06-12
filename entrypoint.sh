#!/bin/sh

if [ -z "${PGID}" ]; then
    PGID="`id -g subscribe`"
fi

if [ -z "${PUID}" ]; then
    PUID="`id -u subscribe`"
fi

if [ -z "${UMASK}" ]; then
    UMASK="022"
fi

if [ -z "${WORK_SPACE}" ]; then
    WORK_SPACE="/app/data"
fi

echo "=================== 启动参数 ==================="
echo "USER_GID = ${PGID}"
echo "USER_UID = ${PUID}"
echo "UMASK = ${UMASK}"
echo "==============================================="


# 更新用户GID?
if [ -n "${PGID}" ] && [ "${PGID}" != "`id -g subscribe`" ]; then
    echo "更新用户GID..."
    sed -i -e "s/^subscribe:\([^:]*\):[0-9]*/subscribe:\1:${PGID}/" /etc/group
    sed -i -e "s/^subscribe:\([^:]*\):\([0-9]*\):[0-9]*/subscribe:\1:\2:${PGID}/" /etc/passwd
fi

# 更新用户UID?
if [ -n "${PUID}" ] && [ "${PUID}" != "`id -u subscribe`" ]; then
    echo "更新用户UID..."
    sed -i -e "s/^subscribe:\([^:]*\):[0-9]*:\([0-9]*\)/subscribe:\1:${PUID}:\2/" /etc/passwd
fi

# 更新umask?
if [ -n "${UMASK}" ]; then
    echo "更新umask..."
    umask ${UMASK}
fi


chown -R subscribe:subscribe /app


# 启动
echo "即将启动..."
cd ${WORK_SPACE}
exec su-exec subscribe /app/mikan-subscriber
