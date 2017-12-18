#pragma once

#include <QString>

class Process
{
public:
    static QByteArray run(const QString &name, const QStringList &args,
                          bool mergeChannels = false,
                          int validExitCode = 0);
};
