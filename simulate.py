import matplotlib.pyplot as plt

E = 1.5
RATE_MODIFY_EXPONENT = 2
MAX_EXPONENT = 47

def rate_modifier(n, slip_rate):
    if n == 0:
        return 0.0
    
    if slip_rate == 0:
        return 1.0

    d = int(n / slip_rate)

    if d > MAX_EXPONENT:
        d = MAX_EXPONENT
    elif d > 0:
        d -= 1
    
    epow = pow(E,-d)

    m = pow(1.0 / (1.0 + epow), RATE_MODIFY_EXPONENT)

    return m

init_rate = 0.1
h = rate_modifier(49,1)
l = rate_modifier(1,1)
x = []
y = []
for i in range(1,50):
    rate = init_rate * rate_modifier(i,1)
    purity = int((h-rate) * 100 / (h - l))
    x.append(i)
    y.append(rate)

plt.plot(x,y, 'r+')
plt.xlabel('supply')
plt.ylabel('purity')
plt.title("A simple line graph")
plt.show()