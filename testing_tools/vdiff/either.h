// Based on https://github.com/robertknight/qt-maybe

#pragma once

#include <QtCore/QMetaType>
#include <QtCore/QVariant>
#include <QtCore/QList>

template <typename T1, typename T2, typename T>
struct EitherHelper
{
};

template <typename T1, typename T2>
struct EitherHelper<T1,T2,T1>
{
	EitherHelper(const QVariant& _value)
        : value(_value)
	{}

	bool is() const
	{
		return value.userType() == qMetaTypeId<T1>();
	}

	T1 as() const {
		return value.value<T1>();
	}

	const QVariant& value;
};

template <typename T1, typename T2>
struct EitherHelper<T1,T2,T2>
{
	EitherHelper(const QVariant& _value)
        : value(_value)
	{}

	bool is() const {
		return value.userType() == qMetaTypeId<T2>();
	}

	T2 as() const {
		return value.value<T2>();
	}

	const QVariant& value;
};

/** A sum type which represents either an instance of T1 or
  * an instance of T2.
  *
  * Either uses QVariant for the underlying storage, so the
  * same performance considerations apply.  An Either instance
  * has the same size as a QVariant (12 bytes) regardless of the
  * types used.
  *
  * A convenient way to create an Either<T1,T2> instance is
  * the some(T) function which returns a type that can be implicitly
  * cast to any Either<> type where T1 or T2 is T.
  */
template <typename T1, typename T2>
class Either
{
	public:
        Either()
        {
        }

		Either(const T1& t1)
            : m_value(QVariant::fromValue(t1))
		{
		}

		Either(const T2& t2)
            : m_value(QVariant::fromValue(t2))
		{
		}

		template <typename T>
		bool is() const;

		template <typename T>
		T as() const;

		bool is1st() const;
		bool is2nd() const;

		T1 as1st() const;
		T2 as2nd() const;

	private:
		template <typename T>
		EitherHelper<T1,T2,T> is_t() const;

		QVariant m_value;
};

template <typename T1, typename T2>
template <typename T>
EitherHelper<T1,T2,T> Either<T1,T2>::is_t() const
{
	return EitherHelper<T1,T2,T>(m_value);
}

template <typename T1, typename T2>
template <typename T>
bool Either<T1, T2>::is() const
{
	return is_t<T>().is();
}

template <typename T1, typename T2>
template <typename T>
T Either<T1, T2>::as() const
{
	Q_ASSERT_X(is<T>(),"Either::as()","Tried to extract the wrong type from an Either instance");
	return is_t<T>().as();
}

template <typename T1, typename T2>
bool Either<T1,T2>::is1st() const
{
	return is<T1>();
}

template <typename T1, typename T2>
bool Either<T1,T2>::is2nd() const
{
	return is<T2>();
}

template <typename T1, typename T2>
T1 Either<T1,T2>::as1st() const
{
	return as<T1>();
}

template <typename T1, typename T2>
T2 Either<T1,T2>::as2nd() const
{
	return as<T2>();
}

template <class T1>
struct MakeEither
{
	MakeEither(const T1& _value)
        : value(_value)
	{}

	template <typename T2>
	operator Either<T1,T2>()
	{
		return Either<T1,T2>(value);
	}

	template <typename T2>
	operator Either<T2,T1>()
	{
		return Either<T2,T1>(value);
	}

	const T1& value;
};

template <class T1>
inline MakeEither<T1> some(const T1& value)
{
	return MakeEither<T1>(value);
}
