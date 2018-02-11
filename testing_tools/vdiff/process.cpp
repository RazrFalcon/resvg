#include <QProcess>

#include "process.h"

QByteArray Process::run(const QString &name, const QStringList &args,
                        bool mergeChannels, int validExitCodes)
{
    QProcess proc;
    if (mergeChannels) {
        proc.setProcessChannelMode(QProcess::MergedChannels);
    }

    proc.start(name, args);

    const QString fullCmd = name + " " + args.join(" ");

    if (!proc.waitForStarted()) {
        throw QString("Process '%1' failed to start.").arg(fullCmd);
    }

    if (!proc.waitForFinished(120000)) { // 2min
        throw QString("Process '%1' was shutdown by timeout.").arg(fullCmd);
    }

    const QByteArray output = proc.readAll();

    if (proc.exitCode() != 0 && proc.exitCode() != validExitCodes) {
        throw QString("Process '%1' finished with an invalid exit code: %2\n%3")
                .arg(name).arg(proc.exitCode()).arg(QString(output));
    }

    if (proc.exitStatus() != QProcess::NormalExit) {
        throw QString("Process '%1' was crashed:\n%2").arg(name).arg(QString(output));
    }

    return output;
}
