import csv
import numpy as np
from matplotlib import pyplot as plt
import seaborn as sns
import sys

n = int(sys.argv[2])

ts = []
labeleds = []
phases = []
with open(sys.argv[1]) as csvfile:
    r = csv.reader(csvfile, delimiter=",")
    for row in r:
        ts.append(int(row[0]))
        labeleds.append(int(row[1]))
        phases.append(float(row[2]))


ts = np.array(ts, dtype=np.float64)
ts /= n * n
labeleds = np.array(labeleds)
phases = np.array(phases)


sns.set(font_scale=0.7)
sns.set_context("paper")
sns.set_theme(rc={'figure.figsize':(3.5,3.5 / 1.8),'font.family':'serif','text.usetex':True,'pgf.rcfonts':False})

fig, ax1 = plt.subplots()

color = 'tab:blue'
ax1.tick_params(axis='x', labelsize=9, pad=-4.0)
ax1.set_xlabel('interactions / $n^2$', fontsize=9, labelpad=-2)
ax1.set_ylabel('number of labeled agents', fontsize=9, labelpad=1, color=color)
ax1.tick_params(axis='y', labelsize=9, pad=0, labelcolor=color)
ax1.set_ylim(0, 250 * 1.05)
ax1.set_xlim(0, ts[-1])
ax1.set_yticks([0, 50, 100, 150, 200, 250])

ax2 = ax1.twinx()  # instantiate a second Axes that shares the same x-axis

color = 'tab:red'
ax2.set_ylabel('average phase', fontsize=9, labelpad=0.0, color=color)  # we already handled the x-label with ax1
ax2.tick_params(axis='y', labelsize=9, pad=2, labelcolor=color)
ax2.set_ylim(0, 10 * 1.05)
ax2.set_yticks([0, 2, 4, 6, 8])

plt.subplots_adjust(left=0.125,bottom=0.114,right=0.91,top=0.99)

ax2.plot(ts, phases, '--', color='tab:red')
ax1.plot(ts, labeleds, '-', color='tab:blue')

plt.savefig("run_256.pgf", format="pgf")

#plt.show()
