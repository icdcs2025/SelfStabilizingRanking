import csv
import numpy as np
from matplotlib import pyplot as plt
import seaborn as sns
import sys

ns = []
ts = []
labeleds = []

with open(sys.argv[1]) as csvfile:
    r = csv.reader(csvfile, delimiter=",")
    for row in r:
        ns.append(int(row[0]))
        ts.append(int(row[1]))
        labeleds.append(int(row[2]))


ns = np.array(ns)
ts = np.array(ts, dtype=np.float64)
ts /= ns * ns
labeleds = np.array(labeleds, dtype=np.float64)
labeleds /= ns
fracts = []


for l in labeleds:
    if l == 0.5:
        fracts.append("1/2")
    elif l == 0.75:
        fracts.append("3/4")
    elif l == 0.875:
        fracts.append("7/8")
    elif l == 0.9375:
        fracts.append("15/16")
    else:
        assert False


sns.set(font_scale=0.7)
sns.set_theme(rc={'figure.figsize':(3.5,3.5 / 1.8),'font.family':'serif','text.usetex':True,'pgf.rcfonts':False})

sns.boxplot(x=ns, y=ts, hue=fracts, linewidth=0.5, flierprops={"marker": ".", "markersize":2})
plt.legend(title="ranked fraction", title_fontsize=9, fontsize=9, labelspacing=-0.1)
plt.ylim(bottom=0)
plt.xlabel('n', fontsize=9, labelpad=-2.0)
plt.ylabel('interactions / n²', fontsize=9, labelpad=-4.0)
plt.tick_params(axis='both', which='major', labelsize=9, pad=-4.0)
plt.subplots_adjust(left=0.068,bottom=0.1,right=0.99,top=0.99)
plt.savefig("ranktimes.pgf", format="pgf")
#plt.show()
