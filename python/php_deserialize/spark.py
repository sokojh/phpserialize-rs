"""
PySpark UDF helpers for PHP deserialization.

This module provides ready-to-use UDFs for deserializing PHP serialized data
in PySpark DataFrames.

Example:
    >>> from php_deserialize.spark import php_deserialize_udf
    >>> deserialize = php_deserialize_udf("json")
    >>> df = df.withColumn("parsed", deserialize("serialized_col"))
"""

from typing import TYPE_CHECKING, Callable, Literal, Optional, Union

if TYPE_CHECKING:
    from pyspark.sql import Column
    from pyspark.sql.types import DataType

# Lazy imports to avoid requiring PySpark at import time
_pyspark_available: Optional[bool] = None


def _check_pyspark() -> None:
    """Check if PySpark is available."""
    global _pyspark_available
    if _pyspark_available is None:
        try:
            import pyspark  # noqa: F401
            _pyspark_available = True
        except ImportError:
            _pyspark_available = False

    if not _pyspark_available:
        raise ImportError(
            "PySpark is required for spark module. "
            "Install with: pip install phpserialize-rs[spark]"
        )


def php_deserialize_udf(
    output_format: Literal["json", "python"] = "json",
    errors: Literal["strict", "replace", "bytes"] = "replace",
    auto_unescape: bool = True,
) -> Callable[["Column"], "Column"]:
    """
    Create a PySpark UDF for deserializing PHP serialized data.

    Args:
        output_format: Output format
            - "json": Return JSON string (recommended for Spark)
            - "python": Return Python object (uses pickle, slower)
        errors: Error handling mode for invalid UTF-8
        auto_unescape: Automatically handle DB-escaped strings

    Returns:
        A UDF function that can be applied to DataFrame columns

    Example:
        >>> from php_deserialize.spark import php_deserialize_udf
        >>> from pyspark.sql import functions as F
        >>>
        >>> deserialize = php_deserialize_udf("json")
        >>> df = df.withColumn("parsed", deserialize(F.col("php_data")))
        >>>
        >>> # Or with schema inference
        >>> from pyspark.sql.functions import from_json, schema_of_json
        >>> sample_json = '{"name":"Alice","age":30}'
        >>> schema = schema_of_json(sample_json)
        >>> df = df.withColumn("parsed", from_json(deserialize("php_data"), schema))
    """
    _check_pyspark()

    from pyspark.sql.functions import udf
    from pyspark.sql.types import StringType

    from php_deserialize import loads, loads_json

    if output_format == "json":
        @udf(returnType=StringType())
        def _deserialize(data: Optional[bytes]) -> Optional[str]:
            if data is None:
                return None
            try:
                # Handle string input (common in Spark)
                if isinstance(data, str):
                    data = data.encode("utf-8")
                return loads_json(data, auto_unescape=auto_unescape)
            except Exception:
                return None

        return _deserialize
    else:
        # For Python output, we need to serialize to JSON anyway
        # because Spark can't handle arbitrary Python objects
        @udf(returnType=StringType())
        def _deserialize(data: Optional[bytes]) -> Optional[str]:
            if data is None:
                return None
            try:
                if isinstance(data, str):
                    data = data.encode("utf-8")
                return loads_json(data, auto_unescape=auto_unescape)
            except Exception:
                return None

        return _deserialize


def php_deserialize_pandas_udf(
    errors: Literal["strict", "replace", "bytes"] = "replace",
    auto_unescape: bool = True,
) -> Callable:
    """
    Create a Pandas UDF for batch PHP deserialization.

    This is more efficient than the regular UDF for large datasets
    as it processes data in batches.

    Args:
        errors: Error handling mode for invalid UTF-8
        auto_unescape: Automatically handle DB-escaped strings

    Returns:
        A Pandas UDF function

    Example:
        >>> from php_deserialize.spark import php_deserialize_pandas_udf
        >>> deserialize = php_deserialize_pandas_udf()
        >>> df = df.withColumn("parsed", deserialize("php_data"))
    """
    _check_pyspark()

    from pyspark.sql.functions import pandas_udf
    from pyspark.sql.types import StringType

    import pandas as pd

    from php_deserialize import loads_json

    @pandas_udf(StringType())
    def _deserialize_batch(series: pd.Series) -> pd.Series:
        def safe_deserialize(data: Optional[Union[bytes, str]]) -> Optional[str]:
            if data is None or (isinstance(data, float) and pd.isna(data)):
                return None
            try:
                if isinstance(data, str):
                    data = data.encode("utf-8")
                return loads_json(data, auto_unescape=auto_unescape)
            except Exception:
                return None

        return series.apply(safe_deserialize)

    return _deserialize_batch


def register_udfs(spark: "SparkSession", prefix: str = "php_") -> None:
    """
    Register PHP deserialization UDFs with a Spark session.

    Args:
        spark: SparkSession instance
        prefix: Prefix for UDF names (default: "php_")

    Example:
        >>> from pyspark.sql import SparkSession
        >>> from php_deserialize.spark import register_udfs
        >>>
        >>> spark = SparkSession.builder.getOrCreate()
        >>> register_udfs(spark)
        >>>
        >>> # Now use in SQL
        >>> spark.sql("SELECT php_deserialize(data) FROM table")
    """
    _check_pyspark()

    from pyspark.sql.types import StringType

    from php_deserialize import loads_json

    def deserialize_udf(data: Optional[bytes]) -> Optional[str]:
        if data is None:
            return None
        try:
            if isinstance(data, str):
                data = data.encode("utf-8")
            return loads_json(data, auto_unescape=True)
        except Exception:
            return None

    spark.udf.register(f"{prefix}deserialize", deserialize_udf, StringType())


# Type alias for Spark
if TYPE_CHECKING:
    from pyspark.sql import SparkSession
